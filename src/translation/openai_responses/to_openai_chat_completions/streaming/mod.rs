//! `openai_responses -> openai_chat_completions` streaming translator.
//!
//! Converts Responses SSE events into Chat Completions stream chunks. Each
//! inbound `ResponseStreamEvent` is mapped to one or more
//! `CreateChatCompletionStreamResponse` chunks via the pure builders in
//! `output`, then converted to carrier-level `StreamEvent`s at the boundary.
//!
//! Source-side lifecycle (identity, terminal detection, unexpected-end
//! reporting) is shared across all `responses -> *` translators through
//! `crate::translation::openai_responses::streaming::ResponsesInboundLifecycle`.

mod output;
mod state;

use crate::protocol::openai::chat_completions::{CompletionUsage, FinishReason};
use crate::protocol::openai::responses::ResponseStreamEvent;
use crate::translation::openai_responses::streaming::ResponsesInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationResult, StreamingEventTranslator,
};

use output::{
    chat_choice_chunk, chat_usage_chunk, message_start_delta, reasoning_delta, text_delta,
    tool_arguments_delta, tool_call_start_delta,
};
use state::StreamingState;

#[derive(Debug, Default)]
pub(super) struct ChatCompletionStreamTranslator {
    lifecycle: ResponsesInboundLifecycle<StreamingState>,
}

impl StreamingEventTranslator for ChatCompletionStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_stream_event(event.data)?;
        let mut chunks = Vec::new();

        match parsed {
            ResponseStreamEvent::ResponseCreated(event) => {
                let identity = response_identity(&event.response);
                self.lifecycle
                    .begin_response_stream(identity.clone(), StreamingState::new())?;
                if self.lifecycle.streaming_state_mut()?.start_message() {
                    chunks.push(StreamEvent::message(chat_choice_chunk(
                        &identity,
                        message_start_delta(),
                        None,
                    ))?);
                }
            }
            ResponseStreamEvent::ResponseInProgress(event) => {
                let identity = response_identity(&event.response);
                self.lifecycle
                    .begin_response_stream(identity.clone(), StreamingState::new())?;
                if self.lifecycle.streaming_state_mut()?.start_message() {
                    chunks.push(StreamEvent::message(chat_choice_chunk(
                        &identity,
                        message_start_delta(),
                        None,
                    ))?);
                }
            }
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                if let crate::protocol::openai::responses::OutputItem::FunctionCall(call) =
                    event.item
                {
                    // Open a Chat tool-call stream only the first time this
                    // output index is seen; later arguments deltas reuse it.
                    if self
                        .lifecycle
                        .streaming_state_mut()?
                        .register_tool_call(event.output_index)
                    {
                        let identity = self.lifecycle.stream_identity()?.clone();
                        chunks.push(StreamEvent::message(chat_choice_chunk(
                            &identity,
                            tool_call_start_delta(event.output_index, call.call_id, call.name),
                            None,
                        ))?);
                    }
                }
            }
            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    text_delta(event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    reasoning_delta(event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    reasoning_delta(event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => {
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    tool_arguments_delta(event.output_index, event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseCompleted(event) => {
                let usage = event.response.usage;
                let finish_reason = self.completed_finish_reason()?;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(finish_reason, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                let usage = event.response.usage;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(FinishReason::Length, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                let usage = event.response.usage;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(FinishReason::Stop, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseError(_) => {
                chunks.extend(self.terminal_chunks(FinishReason::Stop, None)?);
                self.lifecycle.stop();
            }
            other => {
                tracing::trace!(
                    response_stream_event = other.as_ref(),
                    reason = "Responses stream event has no Chat Completions representation"
                );
            }
        }

        Ok(chunks)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_stopped() {
            return Ok(Vec::new());
        }
        // Responses streams always end with a terminal event (completed /
        // incomplete / failed). If we reach here without one, the upstream
        // closed early; surface the unexpected end but emit no synthetic
        // terminal chunk (the protocol gives us no finish_reason to invent).
        tracing::trace!(
            ?end,
            reason = "Responses stream ended without a terminal Responses event"
        );
        let _ = self.lifecycle.unexpected_stream_end_error(end);
        Ok(Vec::new())
    }
}

impl ChatCompletionStreamTranslator {
    fn completed_finish_reason(&mut self) -> StreamTranslationResult<FinishReason> {
        Ok(if self.lifecycle.streaming_state_mut()?.has_tool_calls() {
            FinishReason::ToolCalls
        } else {
            FinishReason::Stop
        })
    }

    /// Build the terminal chunk sequence for a finished Responses turn:
    /// an empty delta carrying the finish reason, an optional usage chunk, and
    /// the carrier-level `[DONE]`.
    fn terminal_chunks(
        &self,
        finish_reason: FinishReason,
        usage: Option<crate::protocol::openai::responses::ResponseUsage>,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let identity = self.lifecycle.stream_identity()?.clone();
        let mut chunks = vec![StreamEvent::message(chat_choice_chunk(
            &identity,
            Default::default(),
            Some(finish_reason),
        ))?];

        if let Some(usage) = usage {
            chunks.push(StreamEvent::message(chat_usage_chunk(
                &identity,
                CompletionUsage::from(&usage),
            ))?);
        }

        chunks.push(StreamEvent::done());
        Ok(chunks)
    }
}

/// Build a `StreamIdentity` from a Responses `Response` snapshot, preserving
/// the upstream `resp_...` id verbatim so round-trip debugging stays tractable.
fn response_identity(response: &crate::protocol::openai::responses::Response) -> StreamIdentity {
    StreamIdentity::new(response.id.clone(), response.model.clone())
}

#[cfg(test)]
mod tests;
