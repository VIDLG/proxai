use axum::body::Bytes;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, InputJsonDelta, MessageDelta, MessageStartEvent,
    MessageStreamEvent, StopReason, TextDelta, ToolUseBlock,
};
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, ChatCompletionStreamResponseDelta,
    CompletionUsage, CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream,
    FunctionType, Role,
};
use crate::sse::{SseEvent, done_sentinel_bytes};
use crate::translation::sse::{
    SseEventTranslator, SseTranslationError, SseTranslationResult, encode_sse_json,
};

#[derive(Debug, Default)]
pub(super) struct AnthropicToChatStreamTranslator {
    id: Option<String>,
    model: Option<String>,
    tool_call_indexes: BTreeMap<u32, u32>,
    next_tool_call_index: u32,
    emitted_text: bool,
}

impl SseEventTranslator for AnthropicToChatStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> SseTranslationResult<Vec<Bytes>> {
        let payload = event.payload_with_type()?;
        if !is_anthropic_stream_event(&payload) {
            return Ok(Vec::new());
        }
        let parsed = serde_json::from_value::<MessageStreamEvent>(payload)?;
        let mut chunks = Vec::new();

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                let delta: ChatCompletionStreamResponseDelta = (&event).into();
                self.id = Some(format!("chatcmpl_{}", event.message.id));
                self.model = Some(event.message.model);
                chunks.push(self.encode_chat_content_chunk(delta, None)?);
            }
            MessageStreamEvent::Ping(_) => {}
            _ if self.id.is_none() => {
                return Err(SseTranslationError::Semantic(
                    "Anthropic stream emitted semantic event before message_start".to_string(),
                ));
            }
            MessageStreamEvent::ContentBlockStart(event) => {
                let index = event.index;
                match event.content_block {
                    ContentBlock::Text(block) if !block.text.is_empty() => {
                        self.emitted_text = true;
                        chunks.push(self.encode_chat_content_chunk(
                            TextDelta { text: block.text }.into(),
                            None,
                        )?);
                    }
                    ContentBlock::ToolUse(block) => {
                        let tool_call_index = self.register_tool_call_index(index);
                        chunks.push(
                            self.encode_chat_content_chunk(
                                ToolStartDelta {
                                    index: tool_call_index,
                                    block,
                                }
                                .into(),
                                None,
                            )?,
                        );
                    }
                    _ => {}
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    if !delta.text.is_empty() {
                        self.emitted_text = true;
                    }
                    chunks.push(self.encode_chat_content_chunk(delta.into(), None)?);
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    if let Some(tool_call_index) = self.tool_call_index(event.index) {
                        chunks.push(
                            self.encode_chat_content_chunk(
                                ToolArgumentsDelta {
                                    index: tool_call_index,
                                    delta,
                                }
                                .into(),
                                None,
                            )?,
                        );
                    }
                }
                _ => {}
            },
            MessageStreamEvent::MessageDelta(event) => {
                if let Some(stop_reason) = event.delta.stop_reason {
                    let delta: ChatCompletionStreamResponseDelta = MessageDeltaWithTextState {
                        delta: event.delta,
                        emitted_text: self.emitted_text,
                    }
                    .into();

                    if delta.refusal.is_some() {
                        chunks.push(self.encode_chat_content_chunk(delta, None)?);
                        chunks.push(self.encode_chat_content_chunk(
                            ChatCompletionStreamResponseDelta::default(),
                            Some(stop_reason.into()),
                        )?);
                    } else {
                        chunks
                            .push(self.encode_chat_content_chunk(delta, Some(stop_reason.into()))?);
                    }

                    // Chat streaming usage is a response-level update. Keep it
                    // in a separate `choices: []` chunk, matching OpenAI's
                    // `stream_options.include_usage` shape, instead of merging it
                    // into a content or terminal choice chunk.
                    chunks
                        .push(self.encode_chat_stream_chunk(Vec::new(), Some(event.usage.into()))?);
                }
            }
            MessageStreamEvent::MessageStop(_) => {
                chunks.push(done_sentinel_bytes());
            }
            MessageStreamEvent::ContentBlockStop(event) => {
                self.clear_tool_call_index(event.index);
            }
        }

        Ok(chunks)
    }
}

impl AnthropicToChatStreamTranslator {
    fn register_tool_call_index(&mut self, block_index: u32) -> u32 {
        let tool_call_index = self.next_tool_call_index;
        self.next_tool_call_index = self.next_tool_call_index.saturating_add(1);
        self.tool_call_indexes.insert(block_index, tool_call_index);
        tool_call_index
    }

    fn tool_call_index(&self, block_index: u32) -> Option<u32> {
        self.tool_call_indexes.get(&block_index).copied()
    }

    fn clear_tool_call_index(&mut self, block_index: u32) {
        self.tool_call_indexes.remove(&block_index);
    }

    fn encode_chat_content_chunk(
        &self,
        delta: ChatCompletionStreamResponseDelta,
        finish_reason: Option<FinishReason>,
    ) -> SseTranslationResult<Bytes> {
        self.encode_chat_stream_chunk(
            vec![ChatChoiceStream {
                index: 0,
                delta,
                finish_reason,
                logprobs: None,
            }],
            None,
        )
    }

    fn encode_chat_stream_chunk(
        &self,
        choices: Vec<ChatChoiceStream>,
        usage: Option<CompletionUsage>,
    ) -> SseTranslationResult<Bytes> {
        let payload = CreateChatCompletionStreamResponse {
            id: self
                .id
                .as_deref()
                .expect("message_start initializes Chat stream id before chunk encoding")
                .to_string(),
            choices,
            created: 0,
            model: self
                .model
                .as_deref()
                .expect("message_start initializes Chat stream model before chunk encoding")
                .to_string(),
            service_tier: None,
            object: "chat.completion.chunk".to_string(),
            usage,
        };
        Ok(encode_sse_json("message", &payload)?)
    }
}

struct ToolStartDelta {
    index: u32,
    block: ToolUseBlock,
}

struct ToolArgumentsDelta {
    index: u32,
    delta: InputJsonDelta,
}

impl From<&MessageStartEvent> for ChatCompletionStreamResponseDelta {
    fn from(_event: &MessageStartEvent) -> Self {
        // Anthropic `message_start` opens the assistant message envelope.
        // Chat Completions streams represent that as the initial role delta.
        Self {
            content: None,
            tool_calls: None,
            role: Some(Role::Assistant),
            refusal: None,
        }
    }
}

impl From<TextDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: TextDelta) -> Self {
        Self {
            content: Some(delta.text),
            tool_calls: None,
            role: None,
            refusal: None,
        }
    }
}

struct MessageDeltaWithTextState {
    delta: MessageDelta,
    emitted_text: bool,
}

impl From<MessageDeltaWithTextState> for ChatCompletionStreamResponseDelta {
    fn from(value: MessageDeltaWithTextState) -> Self {
        let delta = value.delta;
        // MessageDelta.stop_reason is converted by the caller into Chat's
        // choice-level `finish_reason`; Chat stream deltas have no field for
        // Anthropic `container` or `stop_sequence`.
        //
        // Non-streaming refusal conversion can move final text into
        // `message.refusal` and leave `message.content` empty. Streaming cannot
        // retract text deltas that were already sent without buffering the whole
        // response, so only emit `delta.refusal` when no text content has been
        // streamed yet.
        let refusal =
            if !value.emitted_text && matches!(delta.stop_reason, Some(StopReason::Refusal)) {
                delta.stop_details.and_then(|details| details.explanation)
            } else {
                None
            };

        Self {
            refusal,
            ..Self::default()
        }
    }
}

impl From<ToolStartDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: ToolStartDelta) -> Self {
        Self {
            content: None,
            tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
                index: delta.index,
                id: Some(delta.block.id),
                r#type: Some(FunctionType::Function),
                function: Some(FunctionCallStream {
                    name: Some(delta.block.name),
                    // Start the Chat arguments stream with an empty string. `None`
                    // would serialize as JSON null in the local wire model, while
                    // OpenAI-compatible tool argument deltas are string fragments.
                    arguments: Some(String::new()),
                }),
            }]),
            role: None,
            refusal: None,
        }
    }
}

impl From<ToolArgumentsDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: ToolArgumentsDelta) -> Self {
        Self {
            content: None,
            tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
                index: delta.index,
                id: None,
                r#type: None,
                function: Some(FunctionCallStream {
                    name: None,
                    arguments: Some(delta.delta.partial_json),
                }),
            }]),
            role: None,
            refusal: None,
        }
    }
}

fn is_anthropic_stream_event(payload: &Value) -> bool {
    matches!(
        payload.get("type").and_then(Value::as_str),
        Some(
            "ping"
                | "message_start"
                | "content_block_start"
                | "content_block_delta"
                | "content_block_stop"
                | "message_delta"
                | "message_stop"
        )
    )
}
