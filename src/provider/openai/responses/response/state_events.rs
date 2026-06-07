use serde_json::Value;

use crate::protocol::ErrorObject;
use crate::protocol::openai_responses::{
    ResponseErrorEvent, ResponseProjection, ResponseStreamEvent,
};
use crate::sse::SseEvent;

use super::observed::ObservedUpdate;
use super::state::{ResponseSnapshotKind, ResponsesUpstreamState};

enum StateEvent {
    Snapshot {
        kind: ResponseSnapshotKind,
        projection: Box<ResponseProjection>,
    },
    Observed(ObservedUpdate),
    ObservedError(ErrorObject),
    Ignored,
}

impl From<&ResponseStreamEvent> for StateEvent {
    fn from(event: &ResponseStreamEvent) -> Self {
        match event {
            ResponseStreamEvent::ResponseCreated(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::Created,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseInProgress(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::InProgress,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseCompleted(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::Completed,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseFailed(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::Failed,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseIncomplete(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::Incomplete,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseQueued(event) => Self::Snapshot {
                kind: ResponseSnapshotKind::Queued,
                projection: Box::new((&event.response).into()),
            },
            ResponseStreamEvent::ResponseOutputItemAdded(event) => Self::Observed(
                ObservedUpdate::from_output_item(&event.item, event.output_index),
            ),
            ResponseStreamEvent::ResponseOutputItemDone(event) => Self::Observed(
                ObservedUpdate::from_output_item(&event.item, event.output_index),
            ),
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => {
                event.name.as_deref().map_or(Self::Ignored, |name| {
                    Self::Observed(ObservedUpdate::from_function_call_arguments_done(
                        &event.item_id,
                        name,
                    ))
                })
            }
            ResponseStreamEvent::ResponseError(event) => Self::ObservedError(ErrorObject {
                code: event
                    .code
                    .clone()
                    .unwrap_or_else(|| "upstream_error".to_string()),
                message: event.message.clone(),
            }),
            ResponseStreamEvent::ResponseContentPartAdded(_) => Self::Ignored,
            ResponseStreamEvent::ResponseContentPartDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseOutputTextDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseOutputTextDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseRefusalDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseRefusalDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseFileSearchCallInProgress(_) => Self::Ignored,
            ResponseStreamEvent::ResponseFileSearchCallSearching(_) => Self::Ignored,
            ResponseStreamEvent::ResponseFileSearchCallCompleted(_) => Self::Ignored,
            ResponseStreamEvent::ResponseWebSearchCallInProgress(_) => Self::Ignored,
            ResponseStreamEvent::ResponseWebSearchCallSearching(_) => Self::Ignored,
            ResponseStreamEvent::ResponseWebSearchCallCompleted(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningTextDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseReasoningTextDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseImageGenerationCallCompleted(_) => Self::Ignored,
            ResponseStreamEvent::ResponseImageGenerationCallGenerating(_) => Self::Ignored,
            ResponseStreamEvent::ResponseImageGenerationCallInProgress(_) => Self::Ignored,
            ResponseStreamEvent::ResponseImageGenerationCallPartialImage(_) => Self::Ignored,
            ResponseStreamEvent::ResponseMCPCallArgumentsDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseMCPCallArgumentsDone(event) => {
                Self::Observed(ObservedUpdate::from_mcp_call_lifecycle(&event.item_id))
            }
            ResponseStreamEvent::ResponseMCPCallCompleted(event) => {
                Self::Observed(ObservedUpdate::from_mcp_call_lifecycle(&event.item_id))
            }
            ResponseStreamEvent::ResponseMCPCallFailed(event) => {
                Self::Observed(ObservedUpdate::from_mcp_call_lifecycle(&event.item_id))
            }
            ResponseStreamEvent::ResponseMCPCallInProgress(event) => {
                Self::Observed(ObservedUpdate::from_mcp_call_lifecycle(&event.item_id))
            }
            ResponseStreamEvent::ResponseMCPListToolsCompleted(event) => Self::Observed(
                ObservedUpdate::from_mcp_list_tools_lifecycle(&event.item_id),
            ),
            ResponseStreamEvent::ResponseMCPListToolsFailed(event) => Self::Observed(
                ObservedUpdate::from_mcp_list_tools_lifecycle(&event.item_id),
            ),
            ResponseStreamEvent::ResponseMCPListToolsInProgress(event) => Self::Observed(
                ObservedUpdate::from_mcp_list_tools_lifecycle(&event.item_id),
            ),
            ResponseStreamEvent::ResponseCodeInterpreterCallInProgress(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCodeInterpreterCallInterpreting(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCodeInterpreterCallCompleted(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCodeInterpreterCallCodeDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCodeInterpreterCallCodeDone(_) => Self::Ignored,
            ResponseStreamEvent::ResponseOutputTextAnnotationAdded(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCustomToolCallInputDelta(_) => Self::Ignored,
            ResponseStreamEvent::ResponseCustomToolCallInputDone(_) => Self::Ignored,
        }
    }
}

impl ResponsesUpstreamState {
    pub(crate) fn observe_events(&mut self, events: &[SseEvent]) {
        for event in events {
            if let Some(error) = nested_error_event(event) {
                self.record_event(&ResponseStreamEvent::ResponseError(error));
            } else if let Ok(event) = serde_json::from_str::<ResponseStreamEvent>(&event.data) {
                self.record_event(&event.into());
            }
        }
    }

    fn record_event(&mut self, event: &ResponseStreamEvent) {
        self.record_sequence_number(event.sequence_number());

        match StateEvent::from(event) {
            StateEvent::Snapshot { kind, projection } => {
                self.set_snapshot(kind, *projection);
            }
            StateEvent::Observed(update) => {
                self.apply_observed_update(&update);
            }
            StateEvent::ObservedError(error) => self.record_observed_error(error),
            StateEvent::Ignored => {}
        }
    }
}

fn nested_error_event(event: &SseEvent) -> Option<ResponseErrorEvent> {
    let payload = event.payload_json()?;
    if event.event_type != "error" && payload.get("type").and_then(Value::as_str) != Some("error") {
        return None;
    }
    if payload.get("message").and_then(Value::as_str).is_some() {
        return None;
    }

    // This mirrors the outbound SSE compat path. The upstream's hybrid generic
    // error shape is not part of our strict Responses serde model, but the state
    // still needs to recognize it so diagnostics report the actual upstream error
    // instead of leaving the stream stuck at the last `response.created` snapshot.
    let error = payload.get("error")?.as_object()?;
    Some(ResponseErrorEvent {
        sequence_number: payload
            .get("sequence_number")
            .and_then(Value::as_u64)
            .unwrap_or_default(),
        code: error
            .get("code")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        message: error.get("message")?.as_str()?.to_string(),
        param: error
            .get("param")
            .and_then(Value::as_str)
            .map(ToString::to_string),
    })
}

#[cfg(test)]
#[path = "state_events_tests.rs"]
mod tests;
