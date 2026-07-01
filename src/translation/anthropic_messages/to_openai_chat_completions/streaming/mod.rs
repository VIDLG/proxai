//! `anthropic_messages -> openai_chat_completions` streaming translator.
//!
//! This module drives `state::StreamingState` and emits Chat Completions
//! stream chunks built by `output`. It owns no protocol accumulation state
//! of its own beyond the inbound lifecycle wrapper.

use crate::protocol::anthropic::messages::{ContentBlock, ContentBlockDelta, MessageStreamEvent};
use crate::protocol::openai::chat_completions::ChatCompletionStreamResponseDelta;

use crate::translation::anthropic_messages::streaming::AnthropicInboundLifecycle;
use crate::translation::streaming::{
    SseStreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator,
};

mod output;
mod state;

use output::{chat_choice_chunk, chat_terminal_delta, chat_usage_chunk};
use state::{StreamBlock, StreamingState};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[derive(Debug, Default)]
pub(super) struct ChatCompletionStreamTranslator {
    lifecycle: AnthropicInboundLifecycle<StreamingState>,
}

impl StreamingEventTranslator for ChatCompletionStreamTranslator {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_allowed_stream_event(event.data)?;
        let mut chunks = Vec::new();
        let mut done = false;

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                let identity = StreamIdentity::new(
                    format!("chatcmpl_{}", event.message.id),
                    event.message.model,
                );
                self.lifecycle
                    .begin_message_stream(identity.clone(), StreamingState::new())?;
                chunks.push(chat_choice_chunk(
                    &identity,
                    output::message_start_delta(),
                    None,
                ));
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
                            chunks.push(chat_choice_chunk(identity, block.into(), None));
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
                            output::tool_call_start_delta(tool_call_index, block),
                            None,
                        ));
                    }

                    ContentBlock::Thinking(block) => {
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_thinking_block(index)?;
                        if !block.thinking.is_empty() {
                            self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                            let identity = self.lifecycle.stream_identity()?;
                            chunks.push(chat_choice_chunk(identity, block.into(), None));
                        }
                    }
                    ContentBlock::RedactedThinking(_) => {
                        tracing::trace!(
                            block_index = index,
                            "skipping Anthropic redacted_thinking block with no Chat-representable field"
                        );
                        self.lifecycle
                            .streaming_state_mut()?
                            .register_ignored_block(index)?;
                    }

                    ContentBlock::ToolResult(_)
                    | ContentBlock::ServerToolUse(_)
                    | ContentBlock::WebSearchToolResult(_)
                    | ContentBlock::WebFetchToolResult(_)
                    | ContentBlock::CodeExecutionToolResult(_)
                    | ContentBlock::BashCodeExecutionToolResult(_)
                    | ContentBlock::TextEditorCodeExecutionToolResult(_)
                    | ContentBlock::ToolSearchToolResult(_)
                    | ContentBlock::ContainerUpload(_) => {
                        return Err(StreamTranslationError::Semantic(
                            "Anthropic stream emitted content_block_start that Chat Completions cannot represent"
                                .to_string(),
                        ));
                    }
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    self.lifecycle.streaming_state()?.require_block(
                        event.index,
                        StreamBlock::Text,
                        "text_delta",
                    )?;
                    if !delta.text.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_text();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(chat_choice_chunk(identity, delta.into(), None));
                    }
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    let tool_call_index = self
                        .lifecycle
                        .streaming_state()?
                        .get_tool_call_index(event.index)?;

                    let identity = self.lifecycle.stream_identity()?;
                    chunks.push(chat_choice_chunk(
                        identity,
                        output::tool_arguments_delta(tool_call_index, delta.partial_json),
                        None,
                    ));
                }

                ContentBlockDelta::ThinkingDelta(delta) => {
                    self.lifecycle.streaming_state()?.require_block(
                        event.index,
                        StreamBlock::Thinking,
                        "thinking_delta",
                    )?;
                    if !delta.thinking.is_empty() {
                        self.lifecycle.streaming_phase_mut()?.mark_reasoning();
                        let identity = self.lifecycle.stream_identity()?;
                        chunks.push(chat_choice_chunk(identity, delta.into(), None));
                    }
                }
                ContentBlockDelta::SignatureDelta(_) => {
                    self.lifecycle
                        .streaming_state()?
                        .require_reasoning_signature_block(event.index)?;
                }

                ContentBlockDelta::CitationsDelta(_) => {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted content_block_delta that Chat Completions cannot represent"
                            .to_string(),
                    ));
                }
            },
            MessageStreamEvent::MessageDelta(event) => {
                let Some(stop_reason) = event.delta.stop_reason else {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_delta without stop_reason".to_string(),
                    ));
                };

                let mut phase = self.lifecycle.take_streaming_phase()?;
                let emitted_text = phase.emitted_text();
                let emitted_representable_content = phase.emitted_any();
                let terminal_delta = chat_terminal_delta(event.delta, emitted_text);
                let identity = self.lifecycle.stream_identity()?.clone();
                let finish_reason = stop_reason.into();

                if let Some(refusal) = terminal_delta {
                    phase.mark_refusal();
                    chunks.push(chat_choice_chunk(
                        &identity,
                        ChatCompletionStreamResponseDelta {
                            refusal: Some(refusal),
                            ..Default::default()
                        },
                        None,
                    ));
                    chunks.push(chat_choice_chunk(
                        &identity,
                        ChatCompletionStreamResponseDelta::default(),
                        Some(finish_reason),
                    ));
                } else {
                    if !emitted_representable_content {
                        return Err(StreamTranslationError::Semantic(
                            "Anthropic stream completed without Chat-representable content, thinking, refusal, or tool_use blocks"
                                .to_string(),
                        ));
                    }
                    chunks.push(chat_choice_chunk(
                        &identity,
                        ChatCompletionStreamResponseDelta::default(),
                        Some(finish_reason),
                    ));
                }

                // Chat streaming usage is a response-level update. Keep it
                // in a separate `choices: []` chunk, matching OpenAI's
                // `stream_options.include_usage` shape, instead of merging it
                // into a content or terminal choice chunk.
                chunks.push(chat_usage_chunk(&identity, event.usage.into()));

                self.lifecycle.receive_terminal_delta(phase);
            }
            MessageStreamEvent::MessageStop(_) => {
                let _phase = self.lifecycle.take_terminal_phase()?;
                self.lifecycle.stop();
                done = true;
            }
            MessageStreamEvent::ContentBlockStop(event) => {
                self.lifecycle
                    .streaming_state_mut()?
                    .stop_block(event.index)?;
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

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        if self.lifecycle.is_stopped() {
            return Ok(Vec::new());
        }

        Err(self.lifecycle.unexpected_stream_end_error(end))
    }
}
