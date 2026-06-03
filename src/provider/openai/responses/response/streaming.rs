use axum::body::{Body, Bytes};
use axum::http::Response;

use futures_util::Future;

use std::task::Context;
use std::time::Duration;

use super::compat::normalize_nested_error_sse_stream;

use super::{ResponsesUpstreamMetadata, ResponsesUpstreamStreamSnapshot, ResponsesUpstreamTracker};

use crate::http_support::UpstreamResponseHead;
use crate::http_support::response_with_headers;
use crate::observe::{
    ObserveContext, ProviderStreamChunkObserved, ProviderStreamOutcome,
    ProviderStreamOutcomeObserved, ProviderStreamSnapshot,
};
use crate::protocol::ProviderProtocol;
use crate::provider::ProviderStreamingResponsePolicy;

use crate::sse::{SseEventScanner, encode_sse_json_or_error};

use super::sse::is_terminal_event;
use crate::upstream::{
    BodyAction, BodyObserver, UpstreamBodyStreamStats, UpstreamStreamError, prepare_response_stream,
};

use super::tool_arguments::ToolArgumentStreamState;

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

    let stream = Box::pin(normalize_nested_error_sse_stream(body_stream));
    let (status, headers) = outbound_head.into_parts();
    response_with_headers(status, headers, Body::from_stream(stream))
}

struct OpenaiResponsesUpstreamBodyObserver {
    upstream_response_tracker: ResponsesUpstreamTracker,
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
            upstream_response_tracker: ResponsesUpstreamTracker::new(),
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
            state: self.upstream_response_tracker.state.clone(),
            metadata: ResponsesUpstreamMetadata::from_head(head),
        }
    }
}

impl BodyObserver for OpenaiResponsesUpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        self.obs
            .observe_provider_stream_chunk(ProviderStreamChunkObserved {
                provider_protocol: ProviderProtocol::OpenaiResponses,
                chunk,
            });
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

    fn emit_outcome(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = Box::new(self.stream_snapshot(head, stats));

        if let Some(error) = self.stream_error.clone() {
            self.obs
                .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                    snapshot: ProviderStreamSnapshot::OpenaiResponses(&snapshot),
                    outcome: ProviderStreamOutcome::Error(&error),
                });
        } else if self.saw_terminal_event {
            self.obs
                .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                    snapshot: ProviderStreamSnapshot::OpenaiResponses(&snapshot),
                    outcome: ProviderStreamOutcome::Completed,
                });
        } else if self.tool_arguments.has_pending_items() {
            let error = UpstreamStreamError::UnfinishedTool {
                sequence_number: snapshot.state.sequence_number,
            };
            self.obs
                .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                    snapshot: ProviderStreamSnapshot::OpenaiResponses(&snapshot),
                    outcome: ProviderStreamOutcome::UnfinishedTool(&error),
                });
        } else {
            self.obs
                .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                    snapshot: ProviderStreamSnapshot::OpenaiResponses(&snapshot),
                    outcome: ProviderStreamOutcome::Closed,
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
