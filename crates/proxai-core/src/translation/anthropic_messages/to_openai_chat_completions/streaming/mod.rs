//! `anthropic_messages -> openai_chat_completions` streaming translator.
//!
//! This module drives `state::StreamingState` and emits Chat Completions
//! stream chunks built by `output`. It owns no protocol accumulation state
//! of its own beyond the inbound lifecycle wrapper.

use crate::protocol::anthropic::messages::{ContentBlock, ContentBlockDelta, MessageStreamEvent};
use crate::protocol::openai::chat_completions::{
    ChatCompletionStreamResponseDelta, CompletionUsage, FinishReason, FunctionType,
};

use crate::translation::TranslationScope;
use crate::translation::anthropic_messages::streaming::AnthropicInboundLifecycle;
use crate::translation::openai_chat_completions::compatibility::inject_stream_reasoning;
use crate::translation::openai_chat_completions::outbound::{
    assistant_role_delta as message_start_delta, chat_choice_chunk as build_chat_choice_chunk,
    chat_usage_chunk as build_chat_usage_chunk, refusal_delta, tool_arguments_delta,
    tool_call_start_delta,
};
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
};

mod output;
mod state;

use super::types::chat_finish_reason_from_anthropic_stop_reason;
use output::chat_terminal_delta;
use state::StreamingState;

fn chat_choice_chunk(
    identity: &StreamIdentity,
    delta: ChatCompletionStreamResponseDelta,
    finish_reason: Option<FinishReason>,
) -> StreamTranslationResult<serde_json::Value> {
    Ok(serde_json::to_value(build_chat_choice_chunk(
        identity,
        delta,
        finish_reason,
    ))?)
}

fn chat_usage_chunk(
    identity: &StreamIdentity,
    usage: CompletionUsage,
) -> StreamTranslationResult<serde_json::Value> {
    Ok(serde_json::to_value(build_chat_usage_chunk(
        identity, usage,
    ))?)
}

fn chat_reasoning_chunk(
    identity: &StreamIdentity,
    reasoning: String,
) -> StreamTranslationResult<serde_json::Value> {
    let mut payload =
        serde_json::to_value(build_chat_choice_chunk(identity, Default::default(), None))?;
    inject_stream_reasoning(&mut payload, reasoning)?;
    Ok(payload)
}

#[derive(Debug, Default)]
pub(crate) struct AnthropicToChatStreaming {
    lifecycle: AnthropicInboundLifecycle<StreamingState>,
}

impl AnthropicToChatStreaming {
    pub(crate) fn translate_event(
        &mut self,
        event: StreamEvent,
        scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_stream_event(event.data)?;
        let mut chunks = Vec::new();
        let mut done = false;

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                let identity = StreamIdentity::new(
                    format!("chatcmpl_{}", event.message.id),
                    event.message.model,
                );
                self.lifecycle
                    .begin_message_stream(identity.clone(), StreamingState::default())?;
                chunks.push(chat_choice_chunk(&identity, message_start_delta(), None)?);
            }
            MessageStreamEvent::Ping(_) => {}

            MessageStreamEvent::ContentBlockStart(event) => {
                let index = event.index;
                match event.content_block {
                    ContentBlock::Text(block) => {
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_text_block(index)?;
                        if !block.text.is_empty() {
                            self.lifecycle.streaming_phase_mut()?.mark_text();
                            let identity = self.lifecycle.stream_identity()?;
                            chunks.push(chat_choice_chunk(identity, block.into(), None)?);
                        }
                    }
                    ContentBlock::ToolUse(block) => {
                        let tool_call_index = {
                            let state = self.lifecycle.streaming_state_mut()?;
                            state.register_tool_use_block(index)?
                        };
                        self.lifecycle.streaming_phase_mut()?.mark_tool_use();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(chat_choice_chunk(
                            identity,
                            tool_call_start_delta(
                                tool_call_index,
                                block.id,
                                block.name,
                                Some(FunctionType::Function),
                                String::new(),
                            ),
                            None,
                        )?);
                    }

                    ContentBlock::Thinking(block) => {
                        let thinking = block.thinking.clone();
                        let signature = block.signature.clone();
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_thinking_block(index, thinking, signature)?;
                        if !block.thinking.is_empty() {
                            self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                            let identity = self.lifecycle.stream_identity()?;
                            chunks.push(chat_reasoning_chunk(identity, block.thinking)?);
                        }
                    }
                    ContentBlock::RedactedThinking(block) => {
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_redacted_thinking_block(index, block.data.clone())?;
                    }

                    other @ (ContentBlock::ServerToolUse(_)
                    | ContentBlock::WebSearchToolResult(_)
                    | ContentBlock::WebFetchToolResult(_)
                    | ContentBlock::CodeExecutionToolResult(_)
                    | ContentBlock::BashCodeExecutionToolResult(_)
                    | ContentBlock::TextEditorCodeExecutionToolResult(_)
                    | ContentBlock::ToolSearchToolResult(_)
                    | ContentBlock::ContainerUpload(_)) => {
                        let content_block_type = other.as_ref();
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_ignored_block(index, content_block_type)?;
                        scope.dropped(format!("Anthropic content block `{content_block_type}` at index {index}"),
                            "Anthropic content block has no OpenAI Chat Completions stream representation",
                        );
                    }
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    self.lifecycle
                        .streaming_state()?
                        .require_text_block(event.index, "text_delta")?;
                    if !delta.text.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_text();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(chat_choice_chunk(identity, delta.into(), None)?);
                    }
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    let tool_call_index = self
                        .lifecycle
                        .streaming_state()?
                        .require_tool_call_index(event.index)?;

                    let identity = self.lifecycle.stream_identity()?;
                    chunks.push(chat_choice_chunk(
                        identity,
                        tool_arguments_delta(tool_call_index, delta.partial_json),
                        None,
                    )?);
                }

                ContentBlockDelta::ThinkingDelta(delta) => {
                    self.lifecycle
                        .streaming_state_mut()?
                        .append_thinking_delta(event.index, &delta.thinking)?;
                    if !delta.thinking.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(chat_reasoning_chunk(identity, delta.thinking)?);
                    }
                }
                ContentBlockDelta::SignatureDelta(delta) => {
                    self.lifecycle
                        .streaming_state_mut()?
                        .append_signature_delta(event.index, &delta.signature)?;
                }

                ContentBlockDelta::CitationsDelta(_) => {
                    self.lifecycle
                        .streaming_state()?
                        .require_text_block(event.index, "citations_delta")?;
                    scope.dropped(format!("Anthropic citation delta at block index {}", event.index),
                        "Anthropic citation deltas have no OpenAI Chat Completions stream representation",
                    );
                }
            },
            MessageStreamEvent::ContentBlockStop(event) => {
                let produced_continuation = self
                    .lifecycle
                    .streaming_state_mut()?
                    .finish_content_block(event.index)?;
                if produced_continuation {
                    self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                }
            }
            MessageStreamEvent::MessageDelta(event) => {
                let Some(stop_reason) = event.delta.stop_reason.as_non_null().copied() else {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_delta without stop_reason".to_string(),
                    ));
                };

                self.lifecycle
                    .streaming_state()?
                    .ensure_content_blocks_closed()?;

                let mut phase = self.lifecycle.take_streaming_phase()?;
                let emitted_text = phase.emitted_text();
                let emitted_representable_content = phase.emitted_any();
                let terminal_delta = chat_terminal_delta(event.delta, emitted_text);
                let identity = self.lifecycle.stream_identity()?.clone();
                let finish_reason = chat_finish_reason_from_anthropic_stop_reason(stop_reason);

                if let Some(continuation) = phase.state_mut().take_continuation() {
                    chunks.push(chat_reasoning_chunk(
                        &identity,
                        continuation.append_to_chat_reasoning_content(String::new())?,
                    )?);
                }

                if let Some(refusal) = terminal_delta {
                    phase.mark_refusal();
                    chunks.push(chat_choice_chunk(&identity, refusal_delta(refusal), None)?);
                    chunks.push(chat_choice_chunk(
                        &identity,
                        Default::default(),
                        Some(finish_reason),
                    )?);
                } else {
                    if !emitted_representable_content {
                        return Err(StreamTranslationError::Semantic(
                            "Anthropic stream completed without Chat-representable content, thinking, refusal, or tool_use blocks"
                                .to_string(),
                        ));
                    }
                    chunks.push(chat_choice_chunk(
                        &identity,
                        Default::default(),
                        Some(finish_reason),
                    )?);
                }

                // Chat streaming usage is a response-level update. Keep it
                // in a separate `choices: []` chunk, matching OpenAI's
                // `stream_options.include_usage` shape, instead of merging it
                // into a content or terminal choice chunk.
                chunks.push(chat_usage_chunk(&identity, event.usage.into())?);

                self.lifecycle.receive_terminal_delta(phase);
            }
            MessageStreamEvent::MessageStop(_) => {
                let _phase = self.lifecycle.take_terminal_phase()?;
                self.lifecycle.stop();
                done = true;
            }
        }

        let mut events = chunks
            .into_iter()
            .map(StreamEvent::message)
            .collect::<StreamTranslationResult<Vec<_>>>()?;
        if done {
            events.push(StreamEvent::done());
        }
        Ok(events)
    }

    pub(crate) fn finish_stream(
        &mut self,
        end: StreamEnd,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        self.lifecycle.finish_stream(end)?;
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
