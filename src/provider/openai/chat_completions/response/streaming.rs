use axum::body::Body;
use axum::http::Response;

use crate::http_model::UpstreamResponseHead;
use crate::http_utils::response_with_headers;
use crate::logging;
use crate::provider::ProviderStreamingResponseContext;
use crate::upstream::{
    prepare_response_stream, BodyAction, BodyObserver, StreamingResponseContext,
    UpstreamBodyStreamStats, UpstreamStreamError,
};

use super::state::ChatUpstreamStreamSnapshot;
use super::tracker::ChatUpstreamResponseTracker;

pub(crate) async fn handle_streaming_response(
    context: ProviderStreamingResponseContext<'_>,
    response: reqwest::Response,
) -> Response<Body> {
    let ProviderStreamingResponseContext {
        started,
        capture,
        span,
        policy,
        ..
    } = context;
    let head = UpstreamResponseHead::from_response(&response, started.elapsed());
    let observer = ChatUpstreamBodyObserver::new(
        ChatUpstreamResponseTracker::from_headers(&head.headers),
        span.clone(),
    );

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

    span.in_scope(|| {
        logging::ChatLogRecord::Upstream(logging::UpstreamLogRecord::HeadInfo { head: &head })
            .emit()
    });

    response_with_headers(
        head.status,
        outbound_headers,
        Body::from_stream(body_stream),
    )
}

struct ChatUpstreamBodyObserver {
    tracker: ChatUpstreamResponseTracker,
    stream_error: Option<UpstreamStreamError>,
    span: tracing::Span,
}

impl ChatUpstreamBodyObserver {
    fn new(tracker: ChatUpstreamResponseTracker, span: tracing::Span) -> Self {
        Self {
            tracker,
            stream_error: None,
            span,
        }
    }

    fn stream_snapshot(
        &self,
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
    ) -> ChatUpstreamStreamSnapshot {
        ChatUpstreamStreamSnapshot {
            head: head.clone(),
            metrics: stats.metrics(),
            state: self.tracker.state.clone(),
        }
    }
}

impl BodyObserver for ChatUpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        self.tracker.scan_bytes(chunk);
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
                logging::ChatLogRecord::StreamError {
                    snapshot: &snapshot,
                    error,
                }
                .emit()
            });
        } else if self.tracker.state.eof_is_complete() {
            self.span.in_scope(|| {
                logging::ChatLogRecord::Completed {
                    snapshot: &snapshot,
                }
                .emit()
            });
        } else {
            self.span.in_scope(|| {
                logging::ChatLogRecord::Closed {
                    snapshot: &snapshot,
                }
                .emit()
            });
        }
    }
}

#[cfg(test)]
#[path = "handle_tests.rs"]
mod tests;
