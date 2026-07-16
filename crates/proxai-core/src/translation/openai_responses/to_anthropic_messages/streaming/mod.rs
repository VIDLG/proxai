//! `openai_responses -> anthropic_messages` streaming translator.
//!
//! Converts Responses SSE events into Anthropic `MessageStreamEvent`s. Each
//! inbound `ResponseStreamEvent` is mapped to one or more Anthropic
//! `MessageStreamEvent`s, then converted to carrier-level `StreamEvent`s at the boundary.
//!
//! Source-side lifecycle (identity, terminal detection, unexpected-end
//! reporting) is shared across all `responses -> *` translators through
//! `crate::translation::openai_responses::streaming::ResponsesInboundLifecycle`.

mod state;

use crate::protocol::anthropic::messages::{MessageStreamEvent, StopReason};
use crate::protocol::openai_responses::{Response, ResponseStreamEvent};

use crate::translation::openai_responses::streaming::{
    MantleStreamEvent, ResponsesInboundLifecycle, ResponsesInboundStreamEvent,
    response_failure_error,
};

use crate::translation::TranslationScope;
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    typed_stream_events,
};

use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(crate) struct ResponsesToAnthropicStreaming {
    lifecycle: ResponsesInboundLifecycle<StreamingState>,
}

impl ResponsesToAnthropicStreaming {
    pub(crate) fn translate_event(
        &mut self,
        event: StreamEvent,
        scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let inbound = self.lifecycle.parse_stream_event(event.data)?;
        let event_type = inbound.event_type().to_string();
        let mut events = Vec::new();
        let parsed = match inbound {
            ResponsesInboundStreamEvent::Official(event) => *event,
            ResponsesInboundStreamEvent::Mantle(MantleStreamEvent::ReasoningDelta {
                output_index,
                delta,
                ..
            }) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_mantle_reasoning_delta(output_index, delta, &event_type)?,
                );
                return typed_stream_events(events);
            }
            ResponsesInboundStreamEvent::Mantle(MantleStreamEvent::ReasoningDone {
                output_index,
                text,
                ..
            }) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_mantle_reasoning_done(output_index, text, &event_type)?,
                );
                return typed_stream_events(events);
            }
        };

        match parsed {
            ResponseStreamEvent::ResponseCreated(event) => {
                events.extend(self.project_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseInProgress(event) => {
                events.extend(self.project_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.project_output_item_added(
                    event.output_index,
                    event.item,
                    &event_type,
                    scope,
                )?);
            }
            ResponseStreamEvent::ResponseContentPartAdded(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_content_part_added(
                            event.output_index,
                            event.content_index,
                            event.part,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_output_text_delta(
                            event.output_index,
                            event.content_index,
                            event.delta,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseOutputTextDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_output_text_done(
                            event.output_index,
                            event.content_index,
                            event.text,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseRefusalDelta(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_refusal_delta(
                            event.output_index,
                            event.content_index,
                            event.delta,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseRefusalDone(event) => {
                events.extend(self.lifecycle.streaming_state_mut()?.project_refusal_done(
                    event.output_index,
                    event.content_index,
                    event.refusal,
                    &event_type,
                )?);
            }
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => {
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.project_reasoning_text_delta(
                    event.output_index,
                    event.content_index,
                    event.delta,
                    &event_type,
                )?);
            }
            ResponseStreamEvent::ResponseReasoningTextDone(event) => {
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.project_reasoning_text_done(
                    event.output_index,
                    event.content_index,
                    event.text,
                    &event_type,
                )?);
            }

            ResponseStreamEvent::ResponseContentPartDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_content_part_done(
                            event.output_index,
                            event.content_index,
                            event.part,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_reasoning_summary_part_added(
                            event.output_index,
                            event.summary_index,
                            event.part,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_reasoning_summary_text_delta(
                            event.output_index,
                            event.summary_index,
                            event.delta,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_reasoning_summary_text_done(
                            event.output_index,
                            event.summary_index,
                            event.text,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_reasoning_summary_part_done(
                            event.output_index,
                            event.summary_index,
                            event.part,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => {
                let state = self.lifecycle.streaming_state_mut()?;
                events.push(state.project_function_call_arguments_delta(
                    event.output_index,
                    event.delta,
                    &event_type,
                )?);
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_function_call_arguments_done(
                            event.output_index,
                            event.arguments,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseCustomToolCallInputDelta(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .project_custom_tool_call_input_delta(
                        event.output_index,
                        &event.delta,
                        &event_type,
                    )?;
            }
            ResponseStreamEvent::ResponseCustomToolCallInputDone(event) => {
                events.extend(
                    self.lifecycle
                        .streaming_state_mut()?
                        .project_custom_tool_call_input_done(
                            event.output_index,
                            event.input,
                            &event_type,
                        )?,
                );
            }
            ResponseStreamEvent::ResponseOutputItemDone(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .ensure_output_item_projection_closed(event.output_index, &event_type)?;
            }
            ResponseStreamEvent::ResponseCompleted(event) => {
                let usage = event.response.usage.as_ref();
                self.lifecycle.receive_terminal_event()?;
                let state = self.lifecycle.take_terminal_state()?;
                events.extend(state.finish_message(StopReason::EndTurn, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                let usage = event.response.usage.as_ref();
                self.lifecycle.receive_terminal_event()?;
                let state = self.lifecycle.take_terminal_state()?;
                events.extend(state.finish_message(StopReason::MaxTokens, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                return Err(response_failure_error(&event.response));
            }
            ResponseStreamEvent::ResponseError(event) => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream error{}: {}",
                    event
                        .code
                        .as_non_null()
                        .map(String::as_str)
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default(),
                    event.message
                )));
            }
            other => scope.dropped(
                format!("Responses stream event `{}`", other.as_ref()),
                "Responses stream event has no Anthropic Messages representation",
            ),
        }

        typed_stream_events(events)
    }

    pub(crate) fn finish_stream(
        &mut self,
        end: StreamEnd,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        self.lifecycle.finish_stream(end)?;
        Ok(Vec::new())
    }
}

impl ResponsesToAnthropicStreaming {
    fn project_response_snapshot(
        &mut self,
        response: &Response,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let identity = StreamIdentity::new(response.id.clone(), response.model.clone());
        self.lifecycle
            .ensure_response_stream(identity.clone(), StreamingState::default())?;
        Ok(self
            .lifecycle
            .streaming_state_mut()?
            .emit_message_start(&identity, response.usage.as_ref())
            .into_iter()
            .collect())
    }
}
