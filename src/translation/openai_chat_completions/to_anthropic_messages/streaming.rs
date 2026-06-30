use axum::body::Bytes;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent,
    ContentBlockStopEvent, DirectCaller, InputJsonDelta, MessageDelta, MessageDeltaEvent,
    MessageDeltaUsage, MessageStartEvent, MessageStopEvent, MessageStreamEvent, TextBlock,
    TextDelta, ToolCaller, ToolUseBlock, Usage,
};
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, CompletionUsage,
    CreateChatCompletionStreamResponse, FinishReason, Role,
};
use crate::sse::SseEvent;
use crate::translation::openai_chat_completions::stream_lifecycle::{
    ChatInboundLifecycle, ensure_same_stream_identity, stream_identity,
};
use crate::translation::streaming::{
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult,
    StreamingEventTranslator, encode_sse_json,
};

use super::response::chat_stop_state;

#[derive(Debug, Default)]
pub(super) struct MessagesStreamTranslator {
    identity: Option<StreamIdentity>,
    lifecycle: ChatInboundLifecycle<ChatStreamingState, ChatTerminalState>,
}

#[derive(Debug, Default)]
struct ChatToAnthropicBlockState {
    next_block_index: u32,
    text_block_index: Option<u32>,
    tool_block_indexes: BTreeMap<u32, u32>,
}

#[derive(Debug, Default)]
struct ChatStreamingState {
    blocks: ChatToAnthropicBlockState,
    refusal: String,
    choice_index: Option<u32>,
}

impl ChatStreamingState {
    fn text_delta(&mut self, text: String) -> Vec<MessageStreamEvent> {
        match self.blocks.text_block_index {
            Some(index) => vec![MessageStreamEvent::ContentBlockDelta(
                ContentBlockDeltaEvent {
                    index,
                    delta: ContentBlockDelta::TextDelta(TextDelta { text }),
                },
            )],
            None => {
                let index = self.blocks.allocate_block_index();
                self.blocks.text_block_index = Some(index);
                vec![MessageStreamEvent::ContentBlockStart(
                    ContentBlockStartEvent {
                        index,
                        content_block: ContentBlock::Text(TextBlock {
                            citations: None,
                            text,
                        }),
                    },
                )]
            }
        }
    }

    fn refusal_delta(&mut self, refusal: String) -> Vec<MessageStreamEvent> {
        self.refusal.push_str(&refusal);
        self.text_delta(refusal)
    }

    fn tool_call_delta(
        &mut self,
        tool_call: ChatCompletionMessageToolCallChunk,
    ) -> StreamTranslationResult<Vec<MessageStreamEvent>> {
        let arguments = tool_call
            .function
            .as_ref()
            .and_then(|function| function.arguments.clone())
            .filter(|arguments| !arguments.is_empty());

        let mut outputs = Vec::new();
        let block_index = match self
            .blocks
            .tool_block_indexes
            .get(&tool_call.index)
            .copied()
        {
            Some(index) => index,
            None => {
                let id = tool_call.id.clone().ok_or_else(|| {
                    StreamTranslationError::Semantic(
                        "Chat tool call stream started without a tool call id".to_string(),
                    )
                })?;
                let name = tool_call
                    .function
                    .as_ref()
                    .and_then(|function| function.name.clone())
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        StreamTranslationError::Semantic(
                            "Chat tool call stream started without a function name".to_string(),
                        )
                    })?;
                let index = self.blocks.allocate_block_index();
                self.blocks
                    .tool_block_indexes
                    .insert(tool_call.index, index);
                outputs.push(MessageStreamEvent::ContentBlockStart(
                    ContentBlockStartEvent {
                        index,
                        content_block: ContentBlock::ToolUse(ToolUseBlock {
                            id,
                            caller: ToolCaller::Direct(DirectCaller),
                            input: Value::Object(Default::default()),
                            name,
                        }),
                    },
                ));
                index
            }
        };

        if let Some(arguments) = arguments {
            outputs.push(MessageStreamEvent::ContentBlockDelta(
                ContentBlockDeltaEvent {
                    index: block_index,
                    delta: ContentBlockDelta::InputJsonDelta(InputJsonDelta {
                        partial_json: arguments,
                    }),
                },
            ));
        }

        Ok(outputs)
    }

    fn register_choice_index(&mut self, index: u32) -> StreamTranslationResult<()> {
        match self.choice_index {
            Some(existing) if existing != index => Err(StreamTranslationError::Semantic(format!(
                "Chat stream switched from choice index {existing} to {index}; Anthropic message streams can represent exactly one assistant message"
            ))),
            Some(_) => Ok(()),
            None => {
                self.choice_index = Some(index);
                Ok(())
            }
        }
    }
}

impl ChatToAnthropicBlockState {
    fn allocate_block_index(&mut self) -> u32 {
        let index = self.next_block_index;
        self.next_block_index = self.next_block_index.saturating_add(1);
        index
    }

    fn stop_open_blocks(&mut self) -> Vec<MessageStreamEvent> {
        let mut indexes = Vec::new();
        if let Some(index) = self.text_block_index.take() {
            indexes.push(index);
        }
        indexes.extend(self.tool_block_indexes.values().copied());
        self.tool_block_indexes.clear();
        indexes.sort_unstable();

        indexes
            .into_iter()
            .map(|index| MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index }))
            .collect()
    }
}

#[derive(Debug)]
struct ChatTerminalState {
    finish_reason: FinishReason,
    refusal: String,
}

fn encode_outputs(outputs: Vec<MessageStreamEvent>) -> StreamTranslationResult<Vec<Bytes>> {
    outputs
        .into_iter()
        .map(|event| Ok(encode_sse_json(event.as_ref(), &event)?))
        .collect()
}

fn message_delta(
    terminal: &ChatTerminalState,
    usage: Option<&CompletionUsage>,
) -> MessageStreamEvent {
    let usage: Usage = usage.map(Into::into).unwrap_or_default();
    let stop = chat_stop_state(
        (!terminal.refusal.is_empty()).then_some(terminal.refusal.as_str()),
        Some(terminal.finish_reason),
    );
    MessageStreamEvent::MessageDelta(MessageDeltaEvent {
        delta: MessageDelta {
            container: None,
            stop_details: stop.details,
            stop_reason: stop.reason,
            stop_sequence: stop.sequence,
        },
        usage: MessageDeltaUsage {
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            input_tokens: Some(usage.input_tokens),
            output_tokens: usage.output_tokens,
            output_tokens_details: usage.output_tokens_details,
            server_tool_use: usage.server_tool_use,
        },
    })
}

fn chat_choice_stream_identity(chunk: &CreateChatCompletionStreamResponse) -> StreamIdentity {
    stream_identity(chunk, "msg_")
}

fn single_representable_stream_choice(
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

impl StreamingEventTranslator for MessagesStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> StreamTranslationResult<Vec<Bytes>> {
        let payload = event.payload_with_type()?;
        let chunk = serde_json::from_value::<CreateChatCompletionStreamResponse>(payload)?;

        if chunk.choices.is_empty() {
            return encode_outputs(self.translate_usage_only_chunk(&chunk)?);
        }

        let mut outputs = Vec::new();

        if let Some(identity) = self.identity.as_ref() {
            ensure_same_stream_identity(identity, &chunk, "msg_")?;
        } else {
            self.identity = Some(chat_choice_stream_identity(&chunk));
            outputs.push(self.message_start()?);
            self.lifecycle
                .begin_streaming(ChatStreamingState::default());
        }

        let choice = single_representable_stream_choice(chunk.choices)?;

        let phase = self.lifecycle.streaming_phase_mut("Anthropic")?;
        phase.state_mut().register_choice_index(choice.index)?;

        if let Some(content) = choice.delta.content.filter(|content| !content.is_empty()) {
            if !phase.state().refusal.is_empty() {
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
                refusal: std::mem::take(&mut phase.state_mut().refusal),
            };
            self.lifecycle = ChatInboundLifecycle::ReceivedTerminalFinish(terminal);
        }

        encode_outputs(outputs)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<Bytes>> {
        match &self.lifecycle {
            ChatInboundLifecycle::WaitingForFirstChunk => {
                Err(self.lifecycle.unexpected_stream_end_error(end, "Anthropic"))
            }
            ChatInboundLifecycle::Streaming(_) => {
                Err(self.lifecycle.unexpected_stream_end_error(end, "Anthropic"))
            }
            ChatInboundLifecycle::ReceivedTerminalFinish(terminal) => {
                let outputs = vec![
                    message_delta(terminal, None),
                    MessageStreamEvent::MessageStop(MessageStopEvent),
                ];
                self.lifecycle = ChatInboundLifecycle::Stopped;
                encode_outputs(outputs)
            }
            ChatInboundLifecycle::Stopped => Ok(Vec::new()),
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
        let Some(identity) = self.identity.as_ref() else {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted a usage-only chunk before any assistant message chunk"
                    .to_string(),
            ));
        };
        ensure_same_stream_identity(identity, chunk, "msg_")?;

        let outputs = match &self.lifecycle {
            ChatInboundLifecycle::ReceivedTerminalFinish(terminal) => vec![
                message_delta(terminal, Some(usage)),
                MessageStreamEvent::MessageStop(MessageStopEvent),
            ],
            ChatInboundLifecycle::WaitingForFirstChunk => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before any assistant message chunk"
                        .to_string(),
                ));
            }
            ChatInboundLifecycle::Streaming(_) => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before a terminal finish_reason"
                        .to_string(),
                ));
            }
            ChatInboundLifecycle::Stopped => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk after the Anthropic message was stopped"
                        .to_string(),
                ));
            }
        };

        self.lifecycle = ChatInboundLifecycle::Stopped;
        Ok(outputs)
    }

    fn message_start(&self) -> StreamTranslationResult<MessageStreamEvent> {
        let identity = self.identity()?;
        Ok(MessageStreamEvent::MessageStart(
            MessageStartEvent::new_empty_message(
                identity.id().to_string(),
                identity.model().to_string(),
            ),
        ))
    }

    fn identity(&self) -> StreamTranslationResult<&StreamIdentity> {
        self.identity.as_ref().ok_or_else(|| {
            StreamTranslationError::Semantic(
                "Chat stream chunk cannot be encoded before the Anthropic message identity is initialized"
                    .to_string(),
            )
        })
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
