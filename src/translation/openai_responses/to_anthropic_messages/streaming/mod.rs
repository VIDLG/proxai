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

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{MessageStreamEvent, StopReason};
use crate::protocol::openai_responses::{Response, ResponseStreamEvent};

use crate::translation::openai_responses::streaming::{
    ResponsesInboundLifecycle, response_failure_error,
};

use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationError,
    StreamTranslationFailureSink, StreamTranslationResult, StreamingEventTranslator,
    translate_sse_stream, typed_stream_events,
};

use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
pub(super) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_streaming_response_with_failure_sink(input, StreamTranslationFailureSink::default())
}

pub(super) fn translate_streaming_response_with_failure_sink(
    input: ByteStream,
    failure_sink: StreamTranslationFailureSink,
) -> ByteStream {
    translate_sse_stream(input, MessagesStreamTranslator::default(), failure_sink)
}

#[derive(Debug, Default)]
pub(super) struct MessagesStreamTranslator {
    lifecycle: ResponsesInboundLifecycle<StreamingState>,
}

impl StreamingEventTranslator for MessagesStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_stream_event(event.data)?;
        let event_type = parsed.as_ref().to_string();
        let mut events = Vec::new();

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
            other => {
                tracing::trace!(
                    response_stream_event = other.as_ref(),
                    reason = "Responses stream event has no Anthropic Messages representation"
                );
            }
        }

        typed_stream_events(events)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        self.lifecycle.finish_stream(end)?;
        Ok(Vec::new())
    }
}

impl MessagesStreamTranslator {
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
