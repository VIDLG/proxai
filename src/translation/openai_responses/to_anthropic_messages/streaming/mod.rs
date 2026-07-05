//! `openai_responses -> anthropic_messages` streaming translator.
//!
//! Converts Responses SSE events into Anthropic `MessageStreamEvent`s. Each
//! inbound `ResponseStreamEvent` is mapped to one or more Anthropic
//! `MessageStreamEvent`s via the pure builders in `output`, then converted
//! to carrier-level `StreamEvent`s at the boundary.
//!
//! Source-side lifecycle (identity, terminal detection, unexpected-end
//! reporting) is shared across all `responses -> *` translators through
//! `crate::translation::openai_responses::streaming::ResponsesInboundLifecycle`.

mod output;
mod state;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::StopReason;
use crate::protocol::openai_responses::ResponseStreamEvent;
use crate::translation::openai_responses::streaming::ResponsesInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationResult, StreamingEventTranslator,
    translate_sse_stream,
};

use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub(super) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, MessagesStreamTranslator::default())
}

#[derive(Debug, Default)]
pub(super) struct MessagesStreamTranslator {
    lifecycle: ResponsesInboundLifecycle<StreamingState>,
}

impl StreamingEventTranslator for MessagesStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_stream_event(event.data)?;
        let mut events = Vec::new();

        match parsed {
            ResponseStreamEvent::ResponseCreated(event) => {
                let identity = response_identity(&event.response);
                self.lifecycle
                    .begin_response_stream(identity.clone(), StreamingState::default())?;
                self.lifecycle
                    .streaming_state_mut()?
                    .record_usage(&event.response);
                if let Some(event) = self
                    .lifecycle
                    .streaming_state_mut()?
                    .start_message(&identity)
                {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseInProgress(event) => {
                let identity = response_identity(&event.response);
                self.lifecycle
                    .begin_response_stream(identity.clone(), StreamingState::default())?;
                self.lifecycle
                    .streaming_state_mut()?
                    .record_usage(&event.response);
                if let Some(event) = self
                    .lifecycle
                    .streaming_state_mut()?
                    .start_message(&identity)
                {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.register_output_item(event.output_index, event.item) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.ensure_text_block(event.output_index) {
                    events.push(event);
                }
                events.push(output::text_delta(event.output_index, event.delta));
            }
            ResponseStreamEvent::ResponseOutputTextDone(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.stop_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.ensure_thinking_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.stop_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.ensure_thinking_block(event.output_index) {
                    events.push(event);
                }
                events.push(output::thinking_delta(event.output_index, event.delta));
            }
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.ensure_thinking_block(event.output_index) {
                    events.push(event);
                }
                events.push(output::thinking_delta(event.output_index, event.delta));
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.stop_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseReasoningTextDone(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) = state.stop_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) =
                    state.ensure_tool_block(event.output_index, Some(event.item_id.clone()), None)
                {
                    events.push(event);
                }
                events.push(output::input_json_delta(event.output_index, event.delta));
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                let state = self.lifecycle.streaming_state_mut()?;
                events.extend(state.ensure_message_started(&identity));
                if let Some(event) =
                    state.ensure_tool_block(event.output_index, Some(event.item_id), event.name)
                {
                    events.push(event);
                }
                if let Some(event) = state.stop_block(event.output_index) {
                    events.push(event);
                }
            }
            ResponseStreamEvent::ResponseCompleted(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .record_usage(&event.response);
                let identity = self.lifecycle.stream_identity()?.clone();
                self.lifecycle.receive_terminal_event()?;
                let mut state = self.lifecycle.take_terminal_state()?;
                events.extend(state.ensure_message_started(&identity));
                events.extend(state.complete(StopReason::EndTurn));
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .record_usage(&event.response);
                let identity = self.lifecycle.stream_identity()?.clone();
                self.lifecycle.receive_terminal_event()?;
                let mut state = self.lifecycle.take_terminal_state()?;
                events.extend(state.ensure_message_started(&identity));
                events.extend(state.complete(StopReason::MaxTokens));
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .record_usage(&event.response);
                let identity = self.lifecycle.stream_identity()?.clone();
                self.lifecycle.receive_terminal_event()?;
                let mut state = self.lifecycle.take_terminal_state()?;
                events.extend(state.ensure_message_started(&identity));
                events.extend(state.complete(StopReason::Refusal));
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseError(_) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                self.lifecycle.receive_terminal_event()?;
                let mut state = self.lifecycle.take_terminal_state()?;
                events.extend(state.ensure_message_started(&identity));
                events.extend(state.complete(StopReason::Refusal));
                self.lifecycle.stop();
            }
            other => {
                tracing::trace!(
                    response_stream_event = other.as_ref(),
                    reason = "Responses stream event has no Anthropic Messages representation"
                );
            }
        }

        output::encode_events(events)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_stopped() {
            return Ok(Vec::new());
        }
        // Responses streams always end with a terminal event (completed /
        // incomplete / failed). If we reach here without one, the upstream
        // closed early; surface the unexpected end but emit no synthetic
        // terminal events (the protocol gives us no stop_reason to invent).
        tracing::trace!(
            ?end,
            reason = "Responses stream ended without a terminal Responses event"
        );
        let _ = self.lifecycle.unexpected_stream_end_error(end);
        Ok(Vec::new())
    }
}

/// Build a `StreamIdentity` from a Responses `Response` snapshot, preserving
/// the upstream `resp_...` id verbatim so round-trip debugging stays tractable.
fn response_identity(response: &crate::protocol::openai_responses::Response) -> StreamIdentity {
    StreamIdentity::new(response.id.clone(), response.model.clone())
}
