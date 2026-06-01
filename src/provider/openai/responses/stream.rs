use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Response, StatusCode};

use futures_util::{Future, Stream};

use std::task::Context;
use std::time::Duration;

use super::compat::normalize_nested_error_sse_stream;
use super::diagnostic::ResponsesStreamDiagnostics;
use super::result::{emit_responses_stream_result, ResponsesStreamResult};
use super::{ResponsesUpstreamEvent, ResponsesUpstreamStreamSnapshot, ResponsesUpstreamTracker};
use crate::logging;
use crate::provider::{
    build_outbound_stream, BodyAction, BodyObserver, OutboundStream, UpstreamBodyStreamStats,
    UpstreamResponseContext, UpstreamResponseError,
};
use crate::sse::{encode_sse_json_or_error, SseEventScanner};
use crate::upstream::UpstreamResponseHead;

use super::sse::is_terminal_event;
use super::tool_arguments::ToolArgumentStreamState;

const TOOL_ARGUMENT_STALL_MESSAGE: &str = "upstream SSE stalled while streaming tool arguments";

pub(crate) async fn handle_success_response(
    ctx: UpstreamResponseContext<'_>,
    upstream_response: reqwest::Response,
) -> crate::error::Result<Response<Body>> {
    let upstream_headers = upstream_response.headers().clone();

    let observer = OpenaiResponsesUpstreamBodyObserver::new(
        &upstream_headers,
        ctx.sse_tool_call_timeout,
        ctx.request_id,
        ctx.span.clone(),
    );

    let OutboundStream {
        head,
        outbound_headers,
        stream,
        ..
    } = build_outbound_stream(&ctx, upstream_response, observer).await?;

    ctx.span.in_scope(|| {
        logging::ResponsesLogRecord::from_event(&ResponsesUpstreamEvent::Headers {
            head: head.clone(),
        })
        .emit()
    });

    let stream = if head.is_sse() {
        Box::pin(normalize_nested_error_sse_stream(stream))
    } else {
        stream
    };
    Ok(streaming_response(head.status, outbound_headers, stream))
}

struct OpenaiResponsesUpstreamBodyObserver {
    upstream_response_tracker: ResponsesUpstreamTracker,
    outcome: ResponsesStreamOutcome,
    tool_arguments: ToolArgumentStreamState,
    diagnostics: ResponsesStreamDiagnostics,
    timeout: Option<Duration>,
    sse_scanner: SseEventScanner,
    span: tracing::Span,
}

impl OpenaiResponsesUpstreamBodyObserver {
    fn new(
        headers: &HeaderMap,
        timeout: Option<Duration>,
        request_id: u64,
        span: tracing::Span,
    ) -> Self {
        Self {
            upstream_response_tracker: ResponsesUpstreamTracker::from_headers(headers),
            outcome: ResponsesStreamOutcome::default(),
            tool_arguments: ToolArgumentStreamState::default(),
            diagnostics: ResponsesStreamDiagnostics::new(request_id),
            timeout,
            sse_scanner: SseEventScanner::default(),
            span,
        }
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
        }
    }
    fn stream_result(&self, snapshot: &ResponsesUpstreamStreamSnapshot) -> ResponsesStreamResult {
        if let Some(error) = self.outcome.stream_error.clone() {
            ResponsesStreamResult::StreamError { error }
        } else if self
            .upstream_response_tracker
            .state
            .eof_is_complete(self.outcome.saw_terminal_event)
        {
            ResponsesStreamResult::Completed
        } else if self.tool_arguments.has_pending_items() {
            ResponsesStreamResult::UnfinishedTool {
                error: UpstreamResponseError::UnfinishedTool {
                    sequence_number: snapshot.state.sequence_number,
                },
            }
        } else {
            ResponsesStreamResult::Closed
        }
    }
}

#[derive(Default)]
struct ResponsesStreamOutcome {
    saw_terminal_event: bool,
    stream_error: Option<UpstreamResponseError>,
}

impl ResponsesStreamOutcome {
    fn mark_finished(&mut self, tool_arguments: &mut ToolArgumentStreamState) {
        self.saw_terminal_event = true;
        tool_arguments.clear();
    }

    fn mark_error(
        &mut self,
        error: UpstreamResponseError,
        tool_arguments: &mut ToolArgumentStreamState,
    ) {
        self.stream_error = Some(error);
        self.mark_finished(tool_arguments);
    }

    fn mark_error_message(
        &mut self,
        message: impl Into<String>,
        tool_arguments: &mut ToolArgumentStreamState,
    ) {
        self.mark_error(
            UpstreamResponseError::Stream {
                message: message.into(),
            },
            tool_arguments,
        );
    }
}

impl BodyObserver for OpenaiResponsesUpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        self.diagnostics.observe_chunk(chunk);
        self.upstream_response_tracker.scan_bytes(chunk);
        for event in self.sse_scanner.scan(chunk) {
            if is_terminal_event(&event) {
                self.outcome.mark_finished(&mut self.tool_arguments);
                return BodyAction::Continue;
            }
            if let Err(message) = self.tool_arguments.observe_event(&event, self.timeout) {
                self.outcome
                    .mark_error_message(message.clone(), &mut self.tool_arguments);
                return BodyAction::InjectAndClose(error_sse_chunk(
                    self.upstream_response_tracker.state.sequence_number,
                    &message,
                ));
            }
        }
        BodyAction::Continue
    }

    fn observe_error(&mut self, error: &reqwest::Error) {
        self.outcome
            .mark_error_message(error.to_string(), &mut self.tool_arguments);
    }

    fn poll_pending(&mut self, cx: &mut Context<'_>) -> BodyAction {
        let Some(timeout_sleep) = self.tool_arguments.timeout_sleep_mut() else {
            return BodyAction::Continue;
        };

        if timeout_sleep.as_mut().poll(cx).is_pending() {
            return BodyAction::Continue;
        }

        self.outcome.mark_error_message(
            format!(
                "{TOOL_ARGUMENT_STALL_MESSAGE} after {}s",
                self.timeout.unwrap_or_default().as_secs()
            ),
            &mut self.tool_arguments,
        );
        BodyAction::InjectAndClose(error_sse_chunk(
            self.upstream_response_tracker.state.sequence_number,
            TOOL_ARGUMENT_STALL_MESSAGE,
        ))
    }

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = Box::new(self.stream_snapshot(head, stats));
        let result = self.stream_result(&snapshot);
        emit_responses_stream_result(&self.span, &self.diagnostics, snapshot, result);
    }
}

fn streaming_response(
    status: StatusCode,
    headers: HeaderMap,
    stream: impl Stream<Item = std::io::Result<Bytes>> + Send + 'static,
) -> Response<Body> {
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    for (key, value) in headers {
        if let Some(key) = key {
            response.headers_mut().append(key, value);
        }
    }
    response
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
#[path = "stream_tests.rs"]
mod tests;
