use axum::body::{Body, Bytes};
use axum::http::Response;

use futures_util::Future;

use std::task::Context;
use std::time::Duration;

use super::compat::normalize_nested_error_sse_stream;
use super::diagnostic::ResponsesStreamDiagnostics;
use super::{
    ResponsesUpstreamEvent, ResponsesUpstreamMetadata, ResponsesUpstreamStreamSnapshot,
    ResponsesUpstreamTracker,
};

use crate::formatting::compact_tail;
use crate::http_model::UpstreamResponseHead;
use crate::http_utils::response_with_headers;
use crate::logging;
use crate::provider::ProviderStreamingResponseContext;
use crate::request::RequestId;
use crate::sse::{SseEventScanner, encode_sse_json_or_error};

use super::sse::is_terminal_event;
use crate::upstream::{
    BodyAction, BodyObserver, ProgressFields, StreamingResponseContext, UpstreamBodyStreamStats,
    UpstreamStreamError, prepare_response_stream,
};

use super::tool_arguments::ToolArgumentStreamState;

const TOOL_ARGUMENT_STALL_MESSAGE: &str = "upstream SSE stalled while streaming tool arguments";

pub(crate) async fn handle_streaming_response(
    context: ProviderStreamingResponseContext<'_>,
    response: reqwest::Response,
) -> Response<Body> {
    let ProviderStreamingResponseContext {
        request_id,
        started,
        capture,
        span,
        policy,
        ..
    } = context;
    let head = UpstreamResponseHead::from_response(&response, started.elapsed());
    let observer = OpenaiResponsesUpstreamBodyObserver::new(
        policy.sse_tool_call_timeout(),
        request_id,
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
        logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Headers {
            head: head.clone(),
        })
        .emit()
    });

    let stream = Box::pin(normalize_nested_error_sse_stream(body_stream));
    response_with_headers(head.status, outbound_headers, Body::from_stream(stream))
}

struct OpenaiResponsesUpstreamBodyObserver {
    upstream_response_tracker: ResponsesUpstreamTracker,
    saw_terminal_event: bool,
    stream_error: Option<UpstreamStreamError>,
    tool_arguments: ToolArgumentStreamState,
    diagnostics: ResponsesStreamDiagnostics,
    timeout: Option<Duration>,
    sse_scanner: SseEventScanner,
    span: tracing::Span,
}

impl OpenaiResponsesUpstreamBodyObserver {
    fn new(timeout: Option<Duration>, request_id: RequestId, span: tracing::Span) -> Self {
        Self {
            upstream_response_tracker: ResponsesUpstreamTracker::new(),
            saw_terminal_event: false,
            stream_error: None,
            tool_arguments: ToolArgumentStreamState::default(),
            diagnostics: ResponsesStreamDiagnostics::new(request_id),
            timeout,
            sse_scanner: SseEventScanner::default(),
            span,
        }
    }
    fn mark_terminal_event(&mut self) {
        self.saw_terminal_event = true;
        self.tool_arguments.clear();
    }

    fn record_stream_error(&mut self, error: UpstreamStreamError) {
        self.stream_error = Some(error);
        self.mark_terminal_event();
    }

    fn stream_snapshot(
        &self,
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
    ) -> ResponsesUpstreamStreamSnapshot {
        ResponsesUpstreamStreamSnapshot {
            head: head.clone(),
            metrics: stats.metrics(),
            state: self.upstream_response_tracker.state.clone(),
            metadata: ResponsesUpstreamMetadata::from_head(head),
        }
    }
}

impl BodyObserver for OpenaiResponsesUpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        self.diagnostics.observe_chunk(chunk);
        self.upstream_response_tracker.scan_bytes(chunk);
        for event in self.sse_scanner.scan(chunk) {
            if is_terminal_event(&event) {
                self.mark_terminal_event();
                return BodyAction::Continue;
            }
            if let Err(message) = self.tool_arguments.observe_event(&event, self.timeout) {
                self.record_stream_error(UpstreamStreamError::Stream {
                    message: message.clone(),
                });
                return BodyAction::InjectAndClose(error_sse_chunk(
                    self.upstream_response_tracker.state.sequence_number,
                    &message,
                ));
            }
        }
        BodyAction::Continue
    }

    fn observe_error(&mut self, error: &reqwest::Error) {
        self.record_stream_error(UpstreamStreamError::Stream {
            message: error.to_string(),
        });
    }

    fn poll_pending(&mut self, cx: &mut Context<'_>) -> BodyAction {
        let Some(timeout_sleep) = self.tool_arguments.timeout_sleep_mut() else {
            return BodyAction::Continue;
        };

        if timeout_sleep.as_mut().poll(cx).is_pending() {
            return BodyAction::Continue;
        }

        self.record_stream_error(UpstreamStreamError::Stream {
            message: format!(
                "{TOOL_ARGUMENT_STALL_MESSAGE} after {}s",
                self.timeout.unwrap_or_default().as_secs()
            ),
        });
        BodyAction::InjectAndClose(error_sse_chunk(
            self.upstream_response_tracker.state.sequence_number,
            TOOL_ARGUMENT_STALL_MESSAGE,
        ))
    }

    fn progress_fields(&self) -> ProgressFields {
        let state = &self.upstream_response_tracker.state;
        let snapshot = state.latest_snapshot.as_ref();
        ProgressFields {
            phase: if self.tool_arguments.has_pending_items() {
                "tool_args"
            } else {
                "upstream"
            },
            response_id: snapshot.map(|snapshot| compact_tail(&snapshot.projection.id, 8)),
            sequence_number: state.sequence_number,
            response_status: snapshot.map(|snapshot| snapshot.projection.status.to_string()),
            snapshot_kind: snapshot.map(|snapshot| format!("{:?}", snapshot.kind)),
            pending_tool_items: self
                .tool_arguments
                .has_pending_items()
                .then(|| self.tool_arguments.pending_item_count() as u64),
        }
    }

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = Box::new(self.stream_snapshot(head, stats));

        if let Some(error) = self.stream_error.clone() {
            self.span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Error {
                    error,
                    snapshot,
                })
                .emit()
            });
        } else if self.saw_terminal_event {
            self.span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Completed {
                    snapshot,
                })
                .emit()
            });
        } else if self.tool_arguments.has_pending_items() {
            let error = UpstreamStreamError::UnfinishedTool {
                sequence_number: snapshot.state.sequence_number,
            };
            let diagnostic_path = self.diagnostics.write_unfinished_tool_diagnostic(&snapshot);
            self.span.in_scope(|| {
                logging::emit_responses_stream_error_with_diagnostic(
                    &snapshot,
                    &error,
                    diagnostic_path.as_deref(),
                )
            });
        } else {
            self.span.in_scope(|| {
                logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Closed {
                    snapshot,
                })
                .emit()
            });
        }
    }
}

fn error_sse_chunk(sequence_number: Option<u64>, message: &str) -> Bytes {
    encode_sse_json_or_error(
        "error",
        &serde_json::json!({
            "type": "error",
            "sequence_number": sequence_number,
            "code": null,
            "message": message,
            "param": null
        }),
        message,
    )
}

#[cfg(test)]
#[path = "handle_tests.rs"]
mod tests;
