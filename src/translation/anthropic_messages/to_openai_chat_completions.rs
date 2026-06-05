//! `anthropic_messages -> openai_chat_completions` response translation.

use axum::body::Bytes;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, Message, MessageStreamEvent, StopReason,
};
use crate::protocol::openai::chat_completions::{
    ChatChoice, ChatChoiceLogprobs, ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionResponseMessage, CompletionUsage, CreateChatCompletionResponse, FinishReason,
    FunctionCall, Role,
};
use crate::sse::SseEvent;
use crate::translation::TranslationResult;

use crate::http_support::ByteStream;
use crate::translation::sse::{
    SseEventTranslator, SseTranslationResult, encode_sse_json, translate_sse_stream,
};

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, AnthropicToChatStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let message = serde_json::from_value::<Message>(payload)?;
    let translated = translate_message(&message);
    Ok(serde_json::to_value(translated)?)
}

fn translate_message(message: &Message) -> CreateChatCompletionResponse {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for block in &message.content {
        match block {
            ContentBlock::Text(block) => text_parts.push(block.text.clone()),
            ContentBlock::ToolUse(block) => tool_calls.push(
                ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
                    id: block.id.clone(),
                    function: FunctionCall {
                        name: block.name.clone(),
                        arguments: serde_json::to_string(&block.input).unwrap_or_default(),
                    },
                }),
            ),
            _ => {}
        }
    }

    let content = text_parts.join("");
    CreateChatCompletionResponse {
        id: format!("chatcmpl_{}", message.id),
        object: "chat.completion".to_string(),
        created: 0,
        model: message.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatCompletionResponseMessage {
                role: Role::Assistant,
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                refusal: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
                annotations: None,
                audio: None,
            },
            finish_reason: message.stop_reason.map(chat_finish_reason),
            logprobs: None::<ChatChoiceLogprobs>,
        }],
        usage: Some(CompletionUsage {
            prompt_tokens: message.usage.input_tokens,
            completion_tokens: message.usage.output_tokens,
            total_tokens: message
                .usage
                .input_tokens
                .saturating_add(message.usage.output_tokens),
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }),
        service_tier: None,
    }
}

fn chat_finish_reason(stop_reason: StopReason) -> FinishReason {
    match stop_reason {
        StopReason::EndTurn | StopReason::StopSequence => FinishReason::Stop,
        StopReason::MaxTokens => FinishReason::Length,
        StopReason::ToolUse | StopReason::PauseTurn | StopReason::Refusal => {
            FinishReason::ToolCalls
        }
    }
}

#[derive(Debug, Default)]
struct AnthropicToChatStreamTranslator {
    id: String,
    model: String,
    input_tokens: u32,
    output_tokens: u32,
    blocks: BTreeMap<u32, ChatStreamBlock>,
    sent_role: bool,
}

#[derive(Debug, Clone)]
enum ChatStreamBlock {
    Text,
    ToolUse,
    Other,
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
                self.id = format!("chatcmpl_{}", event.message.id);
                self.model = event.message.model;
                self.input_tokens = event.message.usage.input_tokens;
                self.output_tokens = event.message.usage.output_tokens;
                chunks.push(self.chat_chunk(json!({"role": "assistant"}), None)?);
                self.sent_role = true;
            }
            MessageStreamEvent::ContentBlockStart(event) => {
                let index = event.index;
                match event.content_block {
                    ContentBlock::Text(block) => {
                        self.blocks.insert(index, ChatStreamBlock::Text);
                        if !block.text.is_empty() {
                            chunks.push(self.chat_chunk(json!({"content": block.text}), None)?);
                        }
                    }
                    ContentBlock::ToolUse(block) => {
                        let id = block.id.clone();
                        let name = block.name.clone();
                        self.blocks.insert(index, ChatStreamBlock::ToolUse);
                        chunks.push(self.chat_chunk(
                            json!({
                                "tool_calls": [{
                                    "index": index,
                                    "id": id,
                                    "type": "function",
                                    "function": {"name": name, "arguments": ""}
                                }]
                            }),
                            None,
                        )?);
                    }
                    _ => {
                        self.blocks.insert(index, ChatStreamBlock::Other);
                    }
                }
            }
            MessageStreamEvent::ContentBlockDelta(event) => match event.delta {
                ContentBlockDelta::TextDelta(delta) => {
                    chunks.push(self.chat_chunk(json!({"content": delta.text}), None)?);
                }
                ContentBlockDelta::InputJsonDelta(delta) => {
                    if matches!(
                        self.blocks.get(&event.index),
                        Some(ChatStreamBlock::ToolUse)
                    ) {
                        chunks.push(self.chat_chunk(
                            json!({
                                "tool_calls": [{
                                    "index": event.index,
                                    "function": {"arguments": delta.partial_json}
                                }]
                            }),
                            None,
                        )?);
                    }
                }
                _ => {}
            },
            MessageStreamEvent::MessageDelta(event) => {
                self.output_tokens = event.usage.output_tokens;
                if let Some(input_tokens) = event.usage.input_tokens {
                    self.input_tokens = input_tokens;
                }
                if let Some(stop_reason) = event.delta.stop_reason {
                    chunks.push(self.chat_chunk(json!({}), Some(chat_finish_reason(stop_reason)))?);
                }
            }
            MessageStreamEvent::MessageStop(_) => {
                chunks.push(Bytes::from_static(b"data: [DONE]\n\n"));
            }
            MessageStreamEvent::Ping(_) | MessageStreamEvent::ContentBlockStop(_) => {}
        }

        Ok(chunks)
    }
}

impl AnthropicToChatStreamTranslator {
    fn chat_chunk(
        &self,
        delta: Value,
        finish_reason: Option<FinishReason>,
    ) -> SseTranslationResult<Bytes> {
        let payload = json!({
            "id": if self.id.is_empty() { "chatcmpl_stream" } else { self.id.as_str() },
            "object": "chat.completion.chunk",
            "created": 0,
            "model": self.model,
            "choices": [{
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason,
                "logprobs": null
            }]
        });
        Ok(encode_sse_json("message", &payload)?)
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

#[cfg(test)]
#[path = "to_openai_chat_completions_tests.rs"]
mod tests;
