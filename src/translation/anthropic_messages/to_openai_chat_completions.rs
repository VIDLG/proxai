//! `anthropic_messages -> openai_chat_completions` response translation.

use axum::body::{to_bytes, Body, Bytes};
use axum::http::{header, HeaderValue, Response};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io;

use crate::error::{InternalError, Result};
use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, Message, MessageStreamEvent, StopReason,
};
use crate::provider::anthropic_messages;
use crate::sse::SseEvent;
use crate::translation::sse::{
    encode_sse_json, event_payload_with_type, translate_sse_response, SseEventTranslator,
};

pub(crate) async fn translate_response(
    response: Response<Body>,
) -> Result<Response<Body>, InternalError> {
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/event-stream"))
    {
        return Ok(translate_sse_response(
            response,
            AnthropicToChatStreamTranslator::default(),
        ));
    }

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|error| InternalError::Io(std::io::Error::other(error.to_string())))?;
    let payload = serde_json::from_slice::<Value>(&body)?;
    let message = serde_json::from_value::<Message>(
        anthropic_messages::normalize::normalize_message_payload(payload),
    )?;
    let translated = translate_message(&message);
    let mut response = Response::new(Body::from(serde_json::to_vec(&translated)?));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    Ok(response)
}

#[derive(Debug, Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChatChoice>,
    usage: ChatUsage,
}

#[derive(Debug, Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: Option<String>,
    logprobs: Option<Value>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ChatToolCall>>,
}

#[derive(Debug, Clone, Serialize)]
struct ChatToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: ChatFunctionCall,
}

#[derive(Debug, Clone, Serialize)]
struct ChatFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

fn translate_message(message: &Message) -> ChatCompletionResponse {
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    for block in &message.content {
        match block {
            ContentBlock::Text(block) => text_parts.push(block.text.clone()),
            ContentBlock::ToolUse(block) => tool_calls.push(ChatToolCall {
                id: block.id.clone(),
                kind: "function",
                function: ChatFunctionCall {
                    name: block.name.clone(),
                    arguments: serde_json::to_string(&block.input).unwrap_or_default(),
                },
            }),
            _ => {}
        }
    }

    let content = text_parts.join("");
    ChatCompletionResponse {
        id: format!("chatcmpl_{}", message.id),
        object: "chat.completion",
        created: 0,
        model: message.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant",
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
            },
            finish_reason: message.stop_reason.map(chat_finish_reason),
            logprobs: None,
        }],
        usage: ChatUsage {
            prompt_tokens: message.usage.input_tokens,
            completion_tokens: message.usage.output_tokens,
            total_tokens: message
                .usage
                .input_tokens
                .saturating_add(message.usage.output_tokens),
        },
    }
}

fn chat_finish_reason(stop_reason: StopReason) -> String {
    match stop_reason {
        StopReason::EndTurn | StopReason::StopSequence => "stop".to_string(),
        StopReason::MaxTokens => "length".to_string(),
        StopReason::ToolUse | StopReason::PauseTurn | StopReason::Refusal => {
            "tool_calls".to_string()
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
    fn translate_event(&mut self, event: SseEvent) -> io::Result<Vec<Bytes>> {
        let payload = anthropic_messages::normalize::normalize_stream_event_payload(
            event_payload_with_type(&event)?,
        );
        if !is_anthropic_stream_event(&payload) {
            return Ok(Vec::new());
        }
        let parsed =
            serde_json::from_value::<MessageStreamEvent>(payload).map_err(io::Error::other)?;
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
    fn chat_chunk(&self, delta: Value, finish_reason: Option<String>) -> io::Result<Bytes> {
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
        encode_sse_json("message", &payload)
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
