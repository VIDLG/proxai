use axum::body::Body;
use axum::http::Response;

use crate::http_model::UpstreamResponseHead;
use crate::http_utils::response_with_headers;
use crate::logging;
use crate::provider::ProviderStreamingResponseContext;
use crate::upstream::{
    BodyAction, BodyObserver, StreamingResponseContext, UpstreamBodyStreamStats,
    UpstreamStreamError, prepare_response_stream,
};

use super::normalize;
use super::state::AnthropicUpstreamResponseSnapshot;
use super::tracker::AnthropicResponseTracker;

pub(crate) async fn handle_streaming_response(
    context: ProviderStreamingResponseContext<'_>,
    response: reqwest::Response,
) -> Response<Body> {
    let ProviderStreamingResponseContext {
        started,
        capture,
        span,
        policy,
        compatibility,
        ..
    } = context;
    let head = UpstreamResponseHead::from_response(&response, started.elapsed());
    let observer = AnthropicSseObserver::new(AnthropicResponseTracker::new(), span.clone());
    let (outbound_headers, body_stream) = prepare_response_stream(
        StreamingResponseContext {
            capture,
            started,
            span,
            read_idle_timeout: policy.read_idle_timeout(),
            head: &head,
        },
        response,
        observer,
    )
    .await;
    span.in_scope(|| logging::UpstreamLogRecord::HeadInfo { head: &head }.emit());

    if matches!(
        compatibility,
        crate::config::ProviderCompatibility::AnthropicCompatible
    ) {
        return response_with_headers(
            head.status,
            outbound_headers,
            Body::from_stream(normalize::normalize_sse_stream(body_stream)),
        );
    }

    response_with_headers(
        head.status,
        outbound_headers,
        Body::from_stream(body_stream),
    )
}

/// Minimal SSE observer for Anthropic Messages streaming responses.
pub(super) struct AnthropicSseObserver {
    tracker: AnthropicResponseTracker,
    saw_terminal: bool,
    stream_error: Option<UpstreamStreamError>,
    recent_tail: Vec<u8>,
    span: tracing::Span,
}

impl AnthropicSseObserver {
    pub(super) fn new(tracker: AnthropicResponseTracker, span: tracing::Span) -> Self {
        Self {
            tracker,
            saw_terminal: false,
            stream_error: None,
            recent_tail: Vec::new(),
            span,
        }
    }

    fn stream_snapshot(
        &self,
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
    ) -> AnthropicUpstreamResponseSnapshot {
        AnthropicUpstreamResponseSnapshot::streaming(head, stats, self.tracker.state.clone())
    }
}

impl BodyObserver for AnthropicSseObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        const MAX_TAIL: usize = 16 * 1024;
        self.recent_tail.extend_from_slice(chunk);
        if self.recent_tail.len() > MAX_TAIL {
            self.recent_tail.drain(..self.recent_tail.len() - MAX_TAIL);
        }
        self.tracker.scan_bytes(chunk);
        self.saw_terminal |= self.tracker.state.stream_done();
        BodyAction::Continue
    }

    fn observe_error(&mut self, error: &reqwest::Error) {
        self.stream_error = Some(UpstreamStreamError::Stream {
            message: error.to_string(),
        });
    }

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = self.stream_snapshot(head, stats);
        if let Some(ref error) = self.stream_error {
            self.span.in_scope(|| {
                logging::AnthropicLogRecord::StreamError {
                    snapshot: &snapshot,
                    error,
                }
                .emit()
            });
        } else if self.saw_terminal {
            self.span.in_scope(|| {
                logging::AnthropicLogRecord::Completed {
                    snapshot: &snapshot,
                }
                .emit()
            });
        } else {
            self.span.in_scope(|| {
                logging::AnthropicLogRecord::Closed {
                    snapshot: &snapshot,
                }
                .emit()
            });
        }
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
