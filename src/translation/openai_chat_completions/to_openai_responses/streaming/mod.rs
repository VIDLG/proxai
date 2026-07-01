//! `openai_chat_completions -> openai_responses` streaming translator.
//!
//! Drives `state::StreamingState` and emits Responses `ResponseStreamEvent`s
//! (built inline where they depend on streaming context, or via
//! `state::StreamingState` for lifecycle/snapshot events). Maps the
//! resulting events to carrier-level `StreamEvent`s at the boundary.

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;

use crate::translation::openai_chat_completions::streaming::{
    ChatInboundLifecycle, stream_identity,
};
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

mod output;
mod state;
mod types;

use output::{output_text_delta, response_event, tool_arguments_delta};
use state::StreamingState;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(super) struct ResponsesStreamTranslator {
    lifecycle: ChatInboundLifecycle<StreamingState, StreamingState>,
}

impl StreamingEventTranslator for ResponsesStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let chunk = self.lifecycle.parse_stream_event(event.data)?;
        let mut events = Vec::new();

        self.register_chunk_lifecycle(&chunk, &mut events)?;

        if let Some(choice) = chunk.choices.first() {
            let delta = &choice.delta;

            if let Some(content) = delta.content.as_deref()
                && !content.is_empty()
            {
                self.lifecycle.streaming_phase_mut()?.mark_text();
                let state = self.streaming_state_mut()?;
                if let Some(event) = state.ensure_text_item() {
                    events.push(response_event(event)?);
                }
                if let Some((item_id, sequence_number)) = state.append_text_delta(content) {
                    events.push(response_event(output_text_delta(
                        sequence_number,
                        item_id,
                        0,
                        content.to_string(),
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
                            && let Some((item_id, sequence_number)) =
                                state.append_tool_arguments_delta(tool_index, arguments)
                        {
                            events.push(response_event(tool_arguments_delta(
                                sequence_number,
                                item_id,
                                tool_index,
                                arguments.to_string(),
                            ))?);
                        }
                    }
                }
            }

            if choice.finish_reason.is_some() {
                let phase = self.lifecycle.take_streaming_phase(|| {
                    StreamTranslationError::Semantic(
                        "Chat stream emitted terminal finish_reason outside streaming state"
                            .to_string(),
                    )
                })?;
                if !phase.emitted_any() {
                    return Err(StreamTranslationError::Semantic(
                        "Chat stream completed without Responses-representable content or function tool calls"
                            .to_string(),
                    ));
                }
                self.lifecycle.receive_terminal_finish(phase.into_state());
            }
        }

        if let Some(usage) = chunk.usage.clone() {
            self.state_accepting_usage_mut()?.usage = Some(usage);
        }

        Ok(events)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
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

impl ResponsesStreamTranslator {
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
            if chunk.usage.is_none() {
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
            self.lifecycle.terminal_mut().ok_or_else(|| {
                StreamTranslationError::Semantic(
                    "Chat stream usage arrived outside terminal state".to_string(),
                )
            })
        } else {
            self.streaming_state_mut()
        }
    }

    fn finish_completed_stream(&mut self) -> StreamTranslationResult<Vec<StreamEvent>> {
        let mut state = self.lifecycle.take_terminal_finish(|| {
            StreamTranslationError::Semantic(
                "Chat stream completed outside terminal finish_reason state".to_string(),
            )
        })?;
        let events = state.finish_completed_stream();
        self.lifecycle.stop();
        events.into_iter().map(response_event).collect()
    }
}
