use axum::body::Body;
use axum::http::Response;

use crate::logging;
use crate::provider::{
    build_outbound_stream, streaming_response, BodyAction, BodyObserver, OutboundStream,
    UpstreamBodyStreamStats, UpstreamResponseContext,
};
use crate::upstream::UpstreamResponseHead;

use super::state::ChatUpstreamStreamSnapshot;
use super::tracker::ChatUpstreamResponseTracker;

pub(crate) async fn handle_success_response(
    ctx: UpstreamResponseContext<'_>,
    upstream_response: reqwest::Response,
) -> crate::error::Result<Response<Body>> {
    let upstream_headers = upstream_response.headers().clone();

    let observer = ChatUpstreamBodyObserver::new(
        ChatUpstreamResponseTracker::from_headers(&upstream_headers),
        ctx.span.clone(),
    );

    let OutboundStream {
        head,
        outbound_headers,
        stream,
        status,
    } = build_outbound_stream(&ctx, upstream_response, observer).await?;

    ctx.span.in_scope(|| {
        logging::ChatLogRecord::Upstream(logging::UpstreamLogRecord::HeadInfo { head: &head })
            .emit()
    });

    Ok(streaming_response(status, outbound_headers, stream))
}

struct ChatUpstreamBodyObserver {
    tracker: ChatUpstreamResponseTracker,
    error_message: Option<String>,
    span: tracing::Span,
}

impl ChatUpstreamBodyObserver {
    fn new(tracker: ChatUpstreamResponseTracker, span: tracing::Span) -> Self {
        Self {
            tracker,
            error_message: None,
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

    #[cfg(test)]
    fn is_terminal(&self) -> bool {
        self.tracker.state.stream_done
    }

    #[cfg(test)]
    fn is_error(&self) -> bool {
        self.error_message.is_some()
    }
}

impl BodyObserver for ChatUpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        self.tracker.scan_bytes(chunk);
        BodyAction::Continue
    }

    fn observe_error(&mut self, error: &reqwest::Error) {
        self.error_message = Some(error.to_string());
    }

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = self.stream_snapshot(head, stats);
        if let Some(ref message) = self.error_message {
            self.span.in_scope(|| {
                logging::ChatLogRecord::StreamError {
                    snapshot: &snapshot,
                    message,
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
#[path = "stream_wrapper_tests.rs"]
mod tests;
