use axum::body::{Body, Bytes};
use axum::http::Response;
use futures_util::Future;
use std::task::Context;
use std::time::Duration;

use crate::error::ErrorResponseFields;
use crate::http_support::{UpstreamResponseHead, response_with_headers};
use crate::observe::{
    ObserveContext, ProviderStreamOutcome, ProviderStreamOutcomeObserved, ProviderStreamSnapshot,
};

use crate::provider::ProviderStreamingResponsePolicy;
use crate::sse::{SseEventScanner, encode_sse_json};
use crate::upstream::{
    BodyAction, BodyObserver, UpstreamBodyStreamStats, UpstreamStreamError, prepare_response_stream,
};

use super::sse::is_terminal_event;
use super::tool_arguments::ToolArgumentStreamState;
use super::{ResponsesUpstreamMetadata, ResponsesUpstreamState, ResponsesUpstreamStreamSnapshot};

const TOOL_ARGUMENT_STALL_MESSAGE: &str = "upstream SSE stalled while streaming tool arguments";

pub(crate) fn handle_streaming_response(
    obs: &ObserveContext,
    policy: ProviderStreamingResponsePolicy,
    response: reqwest::Response,
) -> Response<Body> {
    let head = UpstreamResponseHead::from_response(&response, obs.elapsed());
    let body_observer =
        OpenaiResponsesUpstreamBodyObserver::new(policy.sse_tool_call_timeout(), (*obs).clone());

    let (outbound_head, body_stream) = prepare_response_stream(
        obs,
        &head,
        policy.read_idle_timeout(),
        response,
        body_observer,
    );

    let (status, headers) = outbound_head.into_parts();
    response_with_headers(status, headers, Body::from_stream(body_stream))
}

struct OpenaiResponsesUpstreamBodyObserver {
    state: ResponsesUpstreamState,
    recent_tail: Vec<u8>,
    saw_terminal_event: bool,
    stream_error: Option<UpstreamStreamError>,
    tool_arguments: ToolArgumentStreamState,
    timeout: Option<Duration>,
    sse_scanner: SseEventScanner,
    obs: crate::observe::ObserveContext,
}

impl OpenaiResponsesUpstreamBodyObserver {
    fn new(timeout: Option<Duration>, obs: crate::observe::ObserveContext) -> Self {
        Self {
            state: ResponsesUpstreamState::default(),
            recent_tail: Vec::new(),
            saw_terminal_event: false,
            stream_error: None,
            tool_arguments: ToolArgumentStreamState::default(),
            timeout,
            sse_scanner: SseEventScanner::default(),
            obs,
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
            state: self.state.clone(),
            recent_tail: self.recent_tail.clone(),
            metadata: ResponsesUpstreamMetadata::from_head(head),
        }
    }
    fn emit_stream_outcome<'a>(
        &self,
        snapshot: &'a ResponsesUpstreamStreamSnapshot,
        outcome: ProviderStreamOutcome<'a>,
    ) {
        self.obs
            .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                snapshot: ProviderStreamSnapshot::OpenaiResponses(snapshot),
                outcome,
            });
    }
}

impl BodyObserver for OpenaiResponsesUpstreamBodyObserver {
    fn on_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        const MAX_STREAM_DIAGNOSTIC_TAIL_BYTES: usize = 16 * 1024;
        self.recent_tail.extend_from_slice(chunk);
        if self.recent_tail.len() > MAX_STREAM_DIAGNOSTIC_TAIL_BYTES {
            let overflow = self.recent_tail.len() - MAX_STREAM_DIAGNOSTIC_TAIL_BYTES;
            self.recent_tail.drain(..overflow);
        }

        let events = self.sse_scanner.scan(chunk);
        self.state.observe_events(&events);
        for event in events {
            if is_terminal_event(&event) {
                self.mark_terminal_event();
                return BodyAction::Continue;
            }
            if let Err(message) = self.tool_arguments.observe_event(&event, self.timeout) {
                self.record_stream_error(UpstreamStreamError::Stream {
                    message: message.clone(),
                });
                return BodyAction::InjectAndClose(error_sse_chunk(
                    self.state.sequence_number,
                    &message,
                ));
            }
        }
        BodyAction::Continue
    }

    fn on_stream_error(&mut self, error: &reqwest::Error) {
        self.record_stream_error(UpstreamStreamError::Stream {
            message: error.to_string(),
        });
    }

    fn poll_pending_action(&mut self, cx: &mut Context<'_>) -> BodyAction {
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
            self.state.sequence_number,
            TOOL_ARGUMENT_STALL_MESSAGE,
        ))
    }

    fn on_stream_finished(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = self.stream_snapshot(head, stats);

        if let Some(error) = self.stream_error.clone() {
            self.emit_stream_outcome(&snapshot, ProviderStreamOutcome::Error(&error));
        } else if self.saw_terminal_event {
            self.emit_stream_outcome(&snapshot, ProviderStreamOutcome::Completed);
        } else if self.tool_arguments.has_pending_items() {
            let error = UpstreamStreamError::UnfinishedTool {
                sequence_number: snapshot.state.sequence_number,
            };
            self.emit_stream_outcome(&snapshot, ProviderStreamOutcome::UnfinishedTool(&error));
        } else {
            self.emit_stream_outcome(&snapshot, ProviderStreamOutcome::Closed);
        }
    }
}

#[derive(serde::Serialize)]
struct ResponsesGenericErrorEvent<'a> {
    #[serde(rename = "type")]
    event_type: &'static str,
    sequence_number: Option<u64>,
    code: Option<&'a str>,
    message: &'a str,
    param: Option<&'a str>,
}

fn error_sse_chunk(sequence_number: Option<u64>, message: &str) -> Bytes {
    // Zed v1.5.3 parses OpenAI Responses `type: "error"` events via
    // `GenericStreamErrorPayload`, accepting either top-level fields or a nested
    // `error` object. Prefer the top-level generic Responses shape here because
    // this error is injected while handling an OpenAI Responses stream.
    encode_sse_json(
        "error",
        &ResponsesGenericErrorEvent {
            event_type: "error",
            sequence_number,
            code: None,
            message,
            param: None,
        },
    )
    .unwrap_or_else(|_| {
        ErrorResponseFields::sse_translation(message).encode_sse_event_or_fallback()
    })
}

#[cfg(test)]
#[path = "handle_tests.rs"]
mod tests;
