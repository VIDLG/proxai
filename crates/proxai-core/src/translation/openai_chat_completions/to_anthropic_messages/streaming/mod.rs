//! `openai_chat_completions -> anthropic_messages` streaming translator.
//!
//! Drives `state::ChatStreamingState` and emits Anthropic
//! `MessageStreamEvent`s built via `output`. Converts the finalized
//! Anthropic events to carrier-level `StreamEvent`s at the boundary.

use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, CreateChatCompletionStreamResponse,
};

use crate::translation::anthropic_messages::outbound::{message_start, message_stop};
use crate::translation::openai_chat_completions::compatibility::stream_reasoning;
use crate::translation::openai_chat_completions::streaming::{
    ChatInboundLifecycle, stream_identity,
};
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamTranslationError, StreamTranslationResult,
    typed_stream_events as encode_outputs,
};

mod output;
mod state;

use state::{ChatStreamingState, PendingAnthropicTerminal};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(crate) struct ChatToAnthropicStreaming {
    lifecycle: ChatInboundLifecycle<ChatStreamingState, PendingAnthropicTerminal>,
}

impl ChatToAnthropicStreaming {
    pub(crate) fn translate_event(
        &mut self,
        event: StreamEvent,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let reasoning = stream_reasoning(&event.data).map_err(StreamTranslationError::Semantic)?;
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
            outputs.push(message_start(
                identity.id().to_string(),
                identity.model().to_string(),
                Default::default(),
            ));
        }

        let choice = single_representable_choice(chunk.choices)?;

        let phase = self.lifecycle.streaming_phase_mut()?;
        phase.state_mut().register_choice_index(choice.index)?;

        if let Some(content) = choice
            .delta
            .content
            .into_non_null()
            .filter(|content| !content.is_empty())
        {
            if phase.state().has_refusal() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            phase.mark_text();
            outputs.extend(phase.state_mut().text_delta(content));
        }
        if let Some(refusal) = choice
            .delta
            .refusal
            .into_non_null()
            .filter(|refusal| !refusal.is_empty())
        {
            if phase.emitted_text() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            phase.mark_refusal();
            outputs.extend(phase.state_mut().refusal_delta(refusal));
        }
        if let Some(reasoning) = reasoning {
            phase.mark_reasoning();
            outputs.extend(phase.state_mut().reasoning_delta(reasoning));
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
        if let Some(finish_reason) = choice.finish_reason.as_non_null().copied() {
            if !phase.emitted_any() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream completed without Anthropic-representable content, refusal, or function tool calls"
                        .to_string(),
                ));
            }
            outputs.extend(phase.state_mut().blocks.stop_open_blocks());
            let terminal = PendingAnthropicTerminal {
                finish_reason,
                refusal: phase.state_mut().take_refusal(),
                usage: chunk.usage.clone().into_non_null(),
            };
            self.lifecycle.receive_terminal_finish(terminal);
        }

        encode_outputs(outputs)
    }

    pub(crate) fn finish_stream(
        &mut self,
        end: StreamEnd,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_waiting_for_first_chunk() {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        } else if let Some(terminal) = self.lifecycle.terminal() {
            let outputs = vec![output::message_delta(terminal), message_stop()];
            self.lifecycle.stop();
            encode_outputs(outputs)
        } else if self.lifecycle.is_stopped() {
            Ok(Vec::new())
        } else {
            Err(self.lifecycle.unexpected_stream_end_error(end))
        }
    }
}

impl ChatToAnthropicStreaming {
    fn translate_usage_only_chunk(
        &mut self,
        chunk: &CreateChatCompletionStreamResponse,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let Some(usage) = chunk.usage.as_non_null() else {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted an empty choices chunk without usage; Anthropic message streams cannot represent it"
                    .to_string(),
            ));
        };
        let identity = stream_identity(chunk, "msg_");
        self.lifecycle.ensure_same_stream_identity(&identity)?;

        let outputs = if self.lifecycle.terminal().is_some() {
            let terminal = self.lifecycle.terminal_mut().ok_or_else(|| {
                StreamTranslationError::Semantic(
                    "Chat stream usage arrived outside terminal state".to_string(),
                )
            })?;
            terminal.usage = Some(usage.clone());
            output::message_delta(terminal)
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
        Ok(vec![outputs, message_stop()])
    }
}

/// Narrow an inbound Chat stream chunk to exactly one choice that can be
/// represented as an Anthropic assistant message.
///
/// Anthropic message streams describe a single assistant turn, so multiple
/// parallel choices and logprobs are rejected as semantically unrepresentable
/// rather than silently dropped. Shared Chat translation lifecycle validation
/// rejects non-assistant roles before this target-specific narrowing.
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
    if choice.logprobs.as_non_null().is_some() {
        return Err(StreamTranslationError::Semantic(
            "Chat stream choice logprobs cannot be represented in Anthropic message streams"
                .to_string(),
        ));
    }

    Ok(choice)
}
