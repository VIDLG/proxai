//! `openai_chat_completions -> openai_responses` streaming translator.
//!
//! Drives `state::StreamingState` and emits Responses `ResponseStreamEvent`s
//! built from accumulated Chat stream state plus shared Responses outbound
//! helpers. Maps the resulting events to carrier-level `StreamEvent`s at the
//! boundary.

use crate::protocol::openai::chat_completions::{CreateChatCompletionStreamResponse, FinishReason};

use crate::translation::TranslationScope;
use crate::translation::openai_chat_completions::compatibility::stream_reasoning;
use crate::translation::openai_chat_completions::streaming::{
    ChatInboundLifecycle, stream_identity,
};
use crate::translation::openai_responses::outbound::{
    output_text_delta, reasoning_text_delta, refusal_delta, tool_arguments_delta,
};
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamTranslationError, StreamTranslationResult,
    typed_stream_event as response_event,
};

mod state;
mod types;

use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug)]
struct PendingResponsesTerminal {
    state: StreamingState,
    finish_reason: FinishReason,
}

#[derive(Debug, Default)]
pub(crate) struct ChatToResponsesStreaming {
    lifecycle: ChatInboundLifecycle<StreamingState, PendingResponsesTerminal>,
}

impl ChatToResponsesStreaming {
    pub(crate) fn translate_event(
        &mut self,
        event: StreamEvent,
        _scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let reasoning = stream_reasoning(&event.data).map_err(StreamTranslationError::Semantic)?;
        let chunk = self.lifecycle.parse_stream_event(event.data)?;
        let mut events = Vec::new();

        self.register_chunk_lifecycle(&chunk, &mut events)?;

        if let Some(choice) = chunk.choices.first() {
            let delta = &choice.delta;

            if let Some(content) = delta.content.as_non_null()
                && !content.is_empty()
            {
                self.lifecycle.streaming_phase_mut()?.mark_text();
                let state = self.streaming_state_mut()?;
                if let Some(event) = state.ensure_text_item()? {
                    events.push(response_event(event)?);
                }
                if let Some((item_id, output_index, sequence_number)) =
                    state.append_text_delta(content)
                {
                    events.push(response_event(output_text_delta(
                        sequence_number,
                        item_id,
                        output_index,
                        content.to_string(),
                    ))?);
                }
            }

            if let Some(refusal) = delta.refusal.as_non_null()
                && !refusal.is_empty()
            {
                self.lifecycle.streaming_phase_mut()?.mark_refusal();
                let state = self.streaming_state_mut()?;
                if let Some(event) = state.ensure_refusal_item()? {
                    events.push(response_event(event)?);
                }
                if let Some((item_id, output_index, sequence_number)) =
                    state.append_refusal_delta(refusal)
                {
                    events.push(response_event(refusal_delta(
                        sequence_number,
                        item_id,
                        output_index,
                        refusal.to_string(),
                    ))?);
                }
            }

            if let Some(reasoning) = reasoning.as_deref() {
                self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                let state = self.streaming_state_mut()?;
                if let Some(event) = state.ensure_reasoning_item() {
                    events.push(response_event(event)?);
                }
                if let Some((item_id, output_index, sequence_number)) =
                    state.append_reasoning_delta(reasoning)
                {
                    events.push(response_event(reasoning_text_delta(
                        sequence_number,
                        item_id,
                        output_index,
                        reasoning.to_string(),
                    ))?);
                }
            }

            if let Some(tool_calls) = delta.tool_calls.as_deref() {
                for tool_call in tool_calls {
                    let tool_index = tool_call.index;
                    self.lifecycle.streaming_phase_mut()?.mark_tool_use();
                    let state = self.streaming_state_mut()?;
                    if let Some(event) = state.ensure_tool_item(tool_index, tool_call)? {
                        events.push(response_event(event)?);
                    }
                    if let Some(function) = tool_call.function.as_ref() {
                        // Defensive: standard OpenAI streams only send `name` on the
                        // first tool_call chunk, but some OpenAI-compatible providers
                        // repeat or correct it on later chunks. Update if present.
                        if let Some(name) = function.name.as_deref() {
                            state.set_tool_name(tool_index, name);
                        }
                        if let Some(arguments) = function.arguments.as_deref()
                            && let Some((item_id, output_index, sequence_number)) =
                                state.append_tool_arguments_delta(tool_index, arguments)
                        {
                            events.push(response_event(tool_arguments_delta(
                                sequence_number,
                                item_id,
                                output_index,
                                arguments.to_string(),
                            ))?);
                        }
                    }
                }
            }

            if let Some(finish_reason) = choice.finish_reason.as_non_null().copied() {
                let phase = self.lifecycle.take_streaming_phase(|| {
                    StreamTranslationError::Semantic(
                        "Chat stream emitted terminal finish_reason outside streaming state"
                            .to_string(),
                    )
                })?;
                if !phase.emitted_any() {
                    return Err(StreamTranslationError::Semantic(
                        "Chat stream completed without Responses-representable content, refusal, reasoning, or function tool calls"
                            .to_string(),
                    ));
                }
                self.lifecycle
                    .receive_terminal_finish(PendingResponsesTerminal {
                        state: phase.into_state(),
                        finish_reason,
                    });
            }
        }

        if let Some(usage) = chunk.usage.as_non_null().cloned() {
            self.state_accepting_usage_mut()?.usage = Some(usage);
        }

        Ok(events)
    }

    pub(crate) fn finish_stream(
        &mut self,
        end: StreamEnd,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_waiting_for_first_chunk() {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        } else if self.lifecycle.terminal().is_some() {
            self.finish_completed_stream()
        } else if self.lifecycle.is_stopped() {
            Ok(Vec::new())
        } else {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        }
    }
}

impl ChatToResponsesStreaming {
    fn register_chunk_lifecycle(
        &mut self,
        chunk: &CreateChatCompletionStreamResponse,
        events: &mut Vec<StreamEvent>,
    ) -> StreamTranslationResult<()> {
        if chunk.choices.is_empty() {
            if self.lifecycle.is_waiting_for_first_chunk() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before any assistant message chunk"
                        .to_string(),
                ));
            }
            if !chunk.usage.is_non_null() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted an empty choices chunk without usage".to_string(),
                ));
            }
            let identity = stream_identity(chunk, "resp_");
            self.lifecycle.ensure_same_stream_identity(&identity)?;
            if !(self.lifecycle.terminal().is_some() || self.lifecycle.is_stopped()) {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before a terminal finish_reason"
                        .to_string(),
                ));
            }
            return Ok(());
        }

        if chunk.choices.len() > 1 {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted multiple choices; Responses translation requires a single assistant choice"
                    .to_string(),
            ));
        }

        let identity = stream_identity(chunk, "resp_");
        let state = StreamingState::new(chunk)?;
        if let Some(_identity) = self.lifecycle.register_chunk_stream(identity, state)? {
            let state = self.streaming_state_mut()?;
            let event = state.response_created_event();
            events.push(response_event(event)?);
        }

        Ok(())
    }

    fn streaming_state_mut(&mut self) -> StreamTranslationResult<&mut StreamingState> {
        Ok(self.lifecycle.streaming_phase_mut()?.state_mut())
    }

    fn state_accepting_usage_mut(&mut self) -> StreamTranslationResult<&mut StreamingState> {
        if self.lifecycle.terminal().is_some() {
            self.lifecycle
                .terminal_mut()
                .map(|terminal| &mut terminal.state)
                .ok_or_else(|| {
                    StreamTranslationError::Semantic(
                        "Chat stream usage arrived outside terminal state".to_string(),
                    )
                })
        } else {
            self.streaming_state_mut()
        }
    }

    fn finish_completed_stream(&mut self) -> StreamTranslationResult<Vec<StreamEvent>> {
        let mut terminal = self.lifecycle.take_terminal_finish(|| {
            StreamTranslationError::Semantic(
                "Chat stream completed outside terminal finish_reason state".to_string(),
            )
        })?;
        let events = terminal.state.finish_stream(terminal.finish_reason);
        self.lifecycle.stop();
        events.into_iter().map(response_event).collect()
    }
}
