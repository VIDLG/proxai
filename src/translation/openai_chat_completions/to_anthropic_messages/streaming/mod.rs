//! `openai_chat_completions -> anthropic_messages` streaming translator.
//!
//! Drives `state::ChatStreamingState` and emits Anthropic
//! `MessageStreamEvent`s built via `output`. Converts the finalized
//! Anthropic events to carrier-level `StreamEvent`s at the boundary.

use crate::protocol::anthropic::messages::{MessageStartEvent, MessageStreamEvent};
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, CreateChatCompletionStreamResponse, Role,
};

use crate::translation::openai_chat_completions::streaming::{
    ChatInboundLifecycle, stream_identity,
};
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

mod output;
mod state;

use output::encode_outputs;
use state::{ChatStreamingState, ChatTerminalState};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(super) struct MessagesStreamTranslator {
    lifecycle: ChatInboundLifecycle<ChatStreamingState, ChatTerminalState>,
}

impl StreamingEventTranslator for MessagesStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let chunk = self.lifecycle.parse_stream_event(event.data)?;

        if chunk.choices.is_empty() {
            return encode_outputs(self.translate_usage_only_chunk(&chunk)?);
        }

        let mut outputs = Vec::new();

        let identity = stream_identity(&chunk, "msg_");
        if let Some(identity) = self
            .lifecycle
            .register_chunk_stream(identity, ChatStreamingState::new())?
        {
            outputs.push(MessageStreamEvent::MessageStart(
                MessageStartEvent::new_empty_message(
                    identity.id().to_string(),
                    identity.model().to_string(),
                ),
            ));
        }

        let choice = single_representable_choice(chunk.choices)?;

        let phase = self.lifecycle.streaming_phase_mut()?;
        phase.state_mut().register_choice_index(choice.index)?;

        if let Some(content) = choice.delta.content.filter(|content| !content.is_empty()) {
            if phase.state().has_refusal() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            phase.mark_text();
            outputs.extend(phase.state_mut().text_delta(content));
        }
        if let Some(refusal) = choice.delta.refusal.filter(|refusal| !refusal.is_empty()) {
            if phase.emitted_text() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            phase.mark_refusal();
            outputs.extend(phase.state_mut().refusal_delta(refusal));
        }
        if let Some(tool_calls) = choice.delta.tool_calls {
            for tool_call in tool_calls {
                let tool_outputs = phase.state_mut().tool_call_delta(tool_call)?;
                if !tool_outputs.is_empty() {
                    phase.mark_tool_use();
                }
                outputs.extend(tool_outputs);
            }
        }
        if let Some(finish_reason) = choice.finish_reason {
            if !phase.emitted_any() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream completed without Anthropic-representable content, refusal, or function tool calls"
                        .to_string(),
                ));
            }
            outputs.extend(phase.state_mut().blocks.stop_open_blocks());
            let terminal = ChatTerminalState {
                finish_reason,
                refusal: phase.state_mut().take_refusal(),
            };
            self.lifecycle.receive_terminal_finish(terminal);
        }

        encode_outputs(outputs)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_waiting_for_first_chunk() {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        } else if let Some(terminal) = self.lifecycle.terminal() {
            let outputs = output::terminal_events(terminal, None);
            self.lifecycle.stop();
            encode_outputs(outputs)
        } else if self.lifecycle.is_stopped() {
            Ok(Vec::new())
        } else {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        }
    }
}

impl MessagesStreamTranslator {
    fn translate_usage_only_chunk(
        &mut self,
        chunk: &CreateChatCompletionStreamResponse,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let Some(usage) = chunk.usage.as_ref() else {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted an empty choices chunk without usage; Anthropic message streams cannot represent it"
                    .to_string(),
            ));
        };
        let identity = stream_identity(chunk, "msg_");
        self.lifecycle.ensure_same_stream_identity(&identity)?;

        let outputs = if let Some(terminal) = self.lifecycle.terminal() {
            output::terminal_events(terminal, Some(usage))
        } else if self.lifecycle.is_waiting_for_first_chunk() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted a usage-only chunk before any assistant message chunk"
                    .to_string(),
            ));
        } else if self.lifecycle.is_stopped() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted a usage-only chunk after the Anthropic message was stopped"
                    .to_string(),
            ));
        } else {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted a usage-only chunk before a terminal finish_reason"
                    .to_string(),
            ));
        };

        self.lifecycle.stop();
        Ok(outputs)
    }
}

/// Narrow an inbound Chat stream chunk to exactly one choice that can be
/// represented as an Anthropic assistant message.
///
/// Anthropic message streams describe a single assistant turn, so multiple
/// parallel choices, logprobs, and non-assistant roles are all rejected as
/// semantically unrepresentable rather than silently dropped.
fn single_representable_choice(
    mut choices: Vec<ChatChoiceStream>,
) -> StreamTranslationResult<ChatChoiceStream> {
    if choices.is_empty() {
        return Err(StreamTranslationError::Semantic(
            "Chat stream chunk has no choices to translate to an Anthropic message event"
                .to_string(),
        ));
    }
    if choices.len() > 1 {
        return Err(StreamTranslationError::Semantic(format!(
            "Chat stream chunk has {} choices; Anthropic message streams can represent exactly one assistant message",
            choices.len()
        )));
    }

    let choice = choices.remove(0);
    if choice.logprobs.is_some() {
        return Err(StreamTranslationError::Semantic(
            "Chat stream choice logprobs cannot be represented in Anthropic message streams"
                .to_string(),
        ));
    }
    match choice.delta.role {
        Some(Role::Assistant) | None => {}
        Some(role) => {
            return Err(StreamTranslationError::Semantic(format!(
                "Chat stream delta role {role} cannot be represented as an Anthropic assistant message"
            )));
        }
    }
    Ok(choice)
}
