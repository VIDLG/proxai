//! `openai_responses -> openai_chat_completions` streaming translator.
//!
//! Converts Responses SSE events into Chat Completions stream chunks. Each
//! inbound `ResponseStreamEvent` is mapped to one or more
//! `CreateChatCompletionStreamResponse` chunks via target Chat outbound helpers,
//! then converted to carrier-level `StreamEvent`s at the boundary.
//!
//! Source-side lifecycle (identity, terminal detection, unexpected-end
//! reporting) is shared across all `responses -> *` translators through
//! `crate::translation::openai_responses::streaming::ResponsesInboundLifecycle`.

mod state;

use crate::protocol::openai::chat_completions::{CompletionUsage, FinishReason};
use crate::protocol::openai::responses::{
    OutputItem, Response, ResponseStreamEvent, ResponseUsage,
};
use crate::translation::openai_chat_completions::outbound::{
    assistant_role_delta as message_start_delta, chat_choice_chunk, chat_usage_chunk,
    refusal_delta, text_delta, tool_arguments_delta, tool_call_start_delta,
};
use crate::translation::openai_responses::streaming::ResponsesInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

use state::StreamingState;

fn reasoning_event(
    identity: &StreamIdentity,
    reasoning: String,
) -> StreamTranslationResult<StreamEvent> {
    let mut payload = serde_json::to_value(chat_choice_chunk(identity, Default::default(), None))?;
    crate::translation::openai_chat_completions::compatibility::inject_stream_reasoning(
        &mut payload,
        reasoning,
    )
    .map_err(|error| StreamTranslationError::Semantic(error.to_string()))?;
    StreamEvent::message(payload)
}

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
                chunks.extend(self.observe_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseInProgress(event) => {
                chunks.extend(self.observe_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                match event.item {
                    OutputItem::FunctionCall(call) => {
                        // Open a Chat tool-call stream only the first time this
                        // output index is seen; later arguments deltas reuse it.
                        if let Some(tool_call_index) = self
                            .lifecycle
                            .streaming_state_mut()?
                            .register_tool_call(event.output_index)
                        {
                            let identity = self.lifecycle.stream_identity()?.clone();
                            chunks.push(StreamEvent::message(chat_choice_chunk(
                                &identity,
                                tool_call_start_delta(
                                    tool_call_index,
                                    call.call_id,
                                    call.name,
                                    None,
                                ),
                                None,
                            ))?);
                        }
                    }
                    OutputItem::Message(_) | OutputItem::Reasoning(_) => {}
                    other => {
                        tracing::trace!(
                            output_index = event.output_index,
                            item_type = other.as_ref(),
                            reason = "Responses output item has no Chat Completions streaming representation",
                            "skipping Responses output item"
                        );
                    }
                }
            }
            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                if event.delta.is_empty() {
                    return Ok(Vec::new());
                }
                self.lifecycle.streaming_state_mut()?.mark_text();
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    text_delta(event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseRefusalDelta(event) => {
                if event.delta.is_empty() {
                    return Ok(Vec::new());
                }
                self.lifecycle.streaming_state_mut()?.mark_refusal();
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    refusal_delta(event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => {
                if event.delta.is_empty() {
                    return Ok(Vec::new());
                }
                self.lifecycle.streaming_state_mut()?.mark_reasoning();
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(reasoning_event(&identity, event.delta)?);
            }
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => {
                if event.delta.is_empty() {
                    return Ok(Vec::new());
                }
                self.lifecycle.streaming_state_mut()?.mark_reasoning();
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(reasoning_event(&identity, event.delta)?);
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => {
                let tool_call_index = self
                    .lifecycle
                    .streaming_state_mut()?
                    .tool_call_index(event.output_index)?;
                let identity = self.lifecycle.stream_identity()?.clone();
                chunks.push(StreamEvent::message(chat_choice_chunk(
                    &identity,
                    tool_arguments_delta(tool_call_index, event.delta),
                    None,
                ))?);
            }
            ResponseStreamEvent::ResponseCompleted(event) => {
                self.require_representable_content()?;
                let usage = event.response.usage;
                let finish_reason = self.completed_finish_reason()?;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(finish_reason, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                self.require_representable_content()?;
                let usage = event.response.usage;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(FinishReason::Length, usage)?);
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
        // Responses streams must end with a terminal semantic event
        // (`response.completed` / `incomplete` / `failed` / `error`). A carrier
        // EOF or `[DONE]` before that means the upstream closed early; surface it
        // as a stream translation error instead of inventing a Chat finish_reason.
        Err(self.lifecycle.unexpected_stream_end_error(end))
    }
}

impl ChatCompletionStreamTranslator {
    fn observe_response_snapshot(
        &mut self,
        response: &Response,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let identity = response_identity(response);
        self.lifecycle
            .ensure_response_stream(identity.clone(), StreamingState::new())?;
        if self.lifecycle.streaming_state_mut()?.start_message() {
            Ok(vec![StreamEvent::message(chat_choice_chunk(
                &identity,
                message_start_delta(),
                None,
            ))?])
        } else {
            Ok(Vec::new())
        }
    }

    fn completed_finish_reason(&mut self) -> StreamTranslationResult<FinishReason> {
        Ok(if self.lifecycle.streaming_state_mut()?.has_tool_calls() {
            FinishReason::ToolCalls
        } else {
            FinishReason::Stop
        })
    }

    fn require_representable_content(&mut self) -> StreamTranslationResult<()> {
        if self.lifecycle.streaming_state_mut()?.emitted_any() {
            Ok(())
        } else {
            Err(StreamTranslationError::Semantic(
                "Responses stream completed without Chat-representable text, refusal, reasoning, or function tool calls"
                    .to_string(),
            ))
        }
    }

    /// Build the terminal chunk sequence for a finished Responses turn:
    /// an empty delta carrying the finish reason, an optional usage chunk, and
    /// the carrier-level `[DONE]`.
    fn terminal_chunks(
        &self,
        finish_reason: FinishReason,
        usage: Option<ResponseUsage>,
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

fn response_failure_error(response: &Response) -> StreamTranslationError {
    let detail = response
        .error
        .as_non_null()
        .map(|error| format!("{}: {}", error.code, error.message))
        .unwrap_or_else(|| "upstream response failed without error details".to_string());
    StreamTranslationError::Semantic(format!("Responses stream failed: {detail}"))
}

/// Build a `StreamIdentity` from a Responses `Response` snapshot, preserving
/// the upstream `resp_...` id verbatim so round-trip debugging stays tractable.
fn response_identity(response: &Response) -> StreamIdentity {
    StreamIdentity::new(response.id.clone(), response.model.clone())
}

#[cfg(test)]
mod tests;
