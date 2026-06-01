use axum::body::Body;
use axum::http::{HeaderMap, Response};

use crate::config::ProviderCompatibility;
use crate::error::Result;
use crate::logging;
use crate::provider::{
    BodyAction, BodyObserver, MonitoredBodyStream, UpstreamBodyStreamStats, UpstreamResponseContext,
};
use crate::upstream::UpstreamResponseHead;

use super::normalize;
use super::snapshot::AnthropicUpstreamResponseSnapshot;
use super::tracker::AnthropicResponseTracker;

pub(super) async fn handle_streaming(
    ctx: UpstreamResponseContext<'_>,
    upstream_response: reqwest::Response,
    upstream_headers: &HeaderMap,
    upstream_head: &UpstreamResponseHead,
    outbound_headers: HeaderMap,
) -> Result<Response<Body>> {
    let observer = AnthropicSseObserver::new(
        AnthropicResponseTracker::from_headers(upstream_headers),
        ctx.span.clone(),
    );

    let stream = MonitoredBodyStream::new(
        upstream_response.bytes_stream(),
        upstream_head.clone(),
        ctx.started,
        observer,
        ctx.capture
            .create_upstream_response_writer(upstream_head.content_type.as_ref()),
    );
    let stream = if should_normalize_provider_response(ctx.provider_compatibility) {
        Body::from_stream(normalize::normalize_sse_stream(stream))
    } else {
        Body::from_stream(stream)
    };

    let mut response = Response::new(stream);
    *response.status_mut() = upstream_head.status;
    *response.headers_mut() = outbound_headers;
    Ok(response)
}

fn should_normalize_provider_response(compatibility: ProviderCompatibility) -> bool {
    matches!(compatibility, ProviderCompatibility::AnthropicCompatible)
}

/// Minimal SSE observer for Anthropic Messages streaming responses.
pub(super) struct AnthropicSseObserver {
    tracker: AnthropicResponseTracker,
    saw_terminal: bool,
    error_message: Option<String>,
    recent_tail: Vec<u8>,
    span: tracing::Span,
}

impl AnthropicSseObserver {
    pub(super) fn new(tracker: AnthropicResponseTracker, span: tracing::Span) -> Self {
        Self {
            tracker,
            saw_terminal: false,
            error_message: None,
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

    #[cfg(test)]
    fn is_terminal(&self) -> bool {
        self.saw_terminal
    }

    #[cfg(test)]
    fn is_error(&self) -> bool {
        self.error_message.is_some()
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
        self.error_message = Some(error.to_string());
    }

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = self.stream_snapshot(head, stats);
        if let Some(ref message) = self.error_message {
            self.span.in_scope(|| {
                logging::AnthropicLogRecord::StreamError {
                    snapshot: &snapshot,
                    message,
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
mod streaming_tests;
