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
use crate::translation::streaming::{
    EmittedContentTracker, SseStreamEnd, StreamIdentity, StreamTranslationError,
    StreamTranslationResult, StreamingEventTranslator, encode_sse_json,
};

use super::response::chat_stop_state;

#[derive(Debug, Default)]
pub(super) struct MessagesStreamTranslator {
    identity: Option<StreamIdentity>,
    lifecycle: ChatStreamLifecycle,
}

#[derive(Debug, Default)]
enum ChatStreamLifecycle {
    #[default]
    WaitingForFirstChunk,
    Streaming(ChatStreamingState),
    ReceivedTerminalFinish(ChatTerminalState),
    Stopped,
}

impl ChatStreamLifecycle {
    fn streaming_state(&mut self) -> StreamTranslationResult<&mut ChatStreamingState> {
        match self {
            ChatStreamLifecycle::WaitingForFirstChunk => Err(StreamTranslationError::Semantic(
                "Chat stream emitted choice deltas before the Anthropic message was initialized"
                    .to_string(),
            )),
            ChatStreamLifecycle::Stopped => Err(StreamTranslationError::Semantic(
                "Chat stream emitted choice deltas after the Anthropic message was stopped"
                    .to_string(),
            )),
            ChatStreamLifecycle::ReceivedTerminalFinish(_) => {
                Err(StreamTranslationError::Semantic(
                    "Chat stream emitted choice deltas after a terminal finish_reason".to_string(),
                ))
            }
            ChatStreamLifecycle::Streaming(state) => Ok(state),
        }
    }
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
    output: EmittedContentTracker,
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
        self.output.mark_refusal();
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
                self.output.mark_tool_use();
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

fn ensure_same_stream_identity(
    identity: &StreamIdentity,
    chunk: &CreateChatCompletionStreamResponse,
) -> StreamTranslationResult<()> {
    let message_id = format!("msg_{}", chunk.id);
    if identity.id() != message_id {
        return Err(StreamTranslationError::Semantic(format!(
            "Chat stream changed id from {} to {message_id}",
            identity.id()
        )));
    }
    if identity.model() != chunk.model {
        return Err(StreamTranslationError::Semantic(format!(
            "Chat stream changed model from {} to {}",
            identity.model(),
            chunk.model
        )));
    }
    Ok(())
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
            ensure_same_stream_identity(identity, &chunk)?;
        } else {
            self.identity = Some(StreamIdentity::new(
                format!("msg_{}", chunk.id),
                chunk.model.clone(),
            ));
            outputs.push(self.message_start()?);
            self.lifecycle = ChatStreamLifecycle::Streaming(ChatStreamingState::default());
        }

        let choice = single_representable_stream_choice(chunk.choices)?;

        let state = self.lifecycle.streaming_state()?;
        state.register_choice_index(choice.index)?;

        if let Some(content) = choice.delta.content.filter(|content| !content.is_empty()) {
            if !state.refusal.is_empty() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            state.output.mark_text();
            outputs.extend(state.text_delta(content));
        }
        if let Some(refusal) = choice.delta.refusal.filter(|refusal| !refusal.is_empty()) {
            if state.output.emitted_text() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream contains both content and refusal deltas; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                        .to_string(),
                ));
            }
            outputs.extend(state.refusal_delta(refusal));
        }
        if let Some(tool_calls) = choice.delta.tool_calls {
            for tool_call in tool_calls {
                outputs.extend(state.tool_call_delta(tool_call)?);
            }
        }
        if let Some(finish_reason) = choice.finish_reason {
            if !state.output.emitted_any() {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream completed without Anthropic-representable content, refusal, or function tool calls"
                        .to_string(),
                ));
            }
            outputs.extend(state.blocks.stop_open_blocks());
            let terminal = ChatTerminalState {
                finish_reason,
                refusal: std::mem::take(&mut state.refusal),
            };
            self.lifecycle = ChatStreamLifecycle::ReceivedTerminalFinish(terminal);
        }

        encode_outputs(outputs)
    }

    fn finish_stream(&mut self, end: SseStreamEnd) -> StreamTranslationResult<Vec<Bytes>> {
        match &self.lifecycle {
            ChatStreamLifecycle::WaitingForFirstChunk => {
                Err(StreamTranslationError::Semantic(match end {
                    SseStreamEnd::DoneSentinel => {
                        "Chat stream emitted [DONE] before any assistant message chunk".to_string()
                    }
                    SseStreamEnd::Eof => {
                        "Chat stream reached EOF before any assistant message chunk".to_string()
                    }
                }))
            }
            ChatStreamLifecycle::Streaming(state) => {
                if !state.output.emitted_any() {
                    return Err(StreamTranslationError::Semantic(
                        "Chat stream completed without Anthropic-representable content, refusal, or function tool calls"
                            .to_string(),
                    ));
                }
                Err(StreamTranslationError::Semantic(match end {
                    SseStreamEnd::DoneSentinel => {
                        "Chat stream emitted [DONE] before a terminal finish_reason".to_string()
                    }
                    SseStreamEnd::Eof => {
                        "Chat stream reached EOF before a terminal finish_reason".to_string()
                    }
                }))
            }
            ChatStreamLifecycle::ReceivedTerminalFinish(terminal) => {
                let outputs = vec![
                    message_delta(terminal, None),
                    MessageStreamEvent::MessageStop(MessageStopEvent),
                ];
                self.lifecycle = ChatStreamLifecycle::Stopped;
                encode_outputs(outputs)
            }
            ChatStreamLifecycle::Stopped => Ok(Vec::new()),
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
        ensure_same_stream_identity(identity, chunk)?;

        let outputs = match &self.lifecycle {
            ChatStreamLifecycle::ReceivedTerminalFinish(terminal) => vec![
                message_delta(terminal, Some(usage)),
                MessageStreamEvent::MessageStop(MessageStopEvent),
            ],
            ChatStreamLifecycle::WaitingForFirstChunk => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before any assistant message chunk"
                        .to_string(),
                ));
            }
            ChatStreamLifecycle::Streaming(_) => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk before a terminal finish_reason"
                        .to_string(),
                ));
            }
            ChatStreamLifecycle::Stopped => {
                return Err(StreamTranslationError::Semantic(
                    "Chat stream emitted a usage-only chunk after the Anthropic message was stopped"
                        .to_string(),
                ));
            }
        };

        self.lifecycle = ChatStreamLifecycle::Stopped;
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
