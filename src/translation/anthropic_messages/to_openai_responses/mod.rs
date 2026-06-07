//! `anthropic_messages -> openai_responses` response translation.

mod types;

use axum::body::Bytes;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, Message, MessageStreamEvent, StopReason,
};
use crate::protocol::openai_responses::{
    AssistantRole, FunctionToolCall, InputTokenDetails, OutputItem, OutputMessage,
    OutputMessageContent, OutputStatus, OutputTextContent, OutputTokenDetails, ReasoningItem,
    Response, ResponseCreateParams, ResponseUsage, Status, SummaryPart, SummaryTextContent,
};
use crate::sse::SseEvent;
use crate::translation::TranslationResult;

use crate::http_support::ByteStream;
use crate::translation::sse::{
    SseEventTranslator, SseTranslationResult, encode_sse_json, translate_sse_stream,
};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request: crate::protocol::anthropic::messages::MessageCreateParamsBase =
        serde_json::from_value(payload.clone())?;
    let mut input = Vec::new();

    for message in request.messages {
        input.push(json!({
            "type": "message",
            "role": match message.role {
                crate::protocol::anthropic::messages::Role::Assistant => "assistant",
                crate::protocol::anthropic::messages::Role::User => "user",
                crate::protocol::anthropic::messages::Role::System => "system",
            },
            "content": match message.content {
                crate::protocol::anthropic::messages::MessageParamContent::Text(text) => {
                    Value::String(text)
                }
                crate::protocol::anthropic::messages::MessageParamContent::Blocks(blocks) => {
                    serde_json::to_value(blocks)?
                }
            }
        }));
    }

    let mut translated = json!({
        "model": request.model,
        "input": input,
        "max_output_tokens": request.max_tokens,
    });

    if let Some(stream) = request.stream {
        translated["stream"] = Value::Bool(stream);
    }
    if let Some(temperature) = request.temperature {
        translated["temperature"] = json!(temperature);
    }
    if let Some(top_p) = request.top_p {
        translated["top_p"] = json!(top_p);
    }
    if let Some(system) = request.system {
        translated["instructions"] = serde_json::to_value(system)?;
    }

    let typed = serde_json::from_value::<ResponseCreateParams>(translated)?;
    Ok(serde_json::to_value(typed)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, AnthropicToOpenaiStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let message = serde_json::from_value::<Message>(payload)?;
    let translated = translate_message(&message)?;
    Ok(serde_json::to_value(translated)?)
}

#[derive(Debug, Default)]
struct AnthropicToOpenaiStreamTranslator {
    sequence_number: u64,
    response_id: Option<String>,
    model: Option<String>,
    input_tokens: u32,
    output_tokens: u32,
    blocks: BTreeMap<u32, AnthropicStreamBlock>,
    completed: bool,
}

#[derive(Debug, Clone)]
enum AnthropicStreamBlock {
    Text {
        item_id: String,
        content_index: u32,
        text: String,
    },
    Thinking {
        item_id: String,
        summary_index: u32,
        text: String,
    },
    ToolUse {
        item_id: String,
        name: String,
        arguments: String,
    },
    Other,
}

impl SseEventTranslator for AnthropicToOpenaiStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> SseTranslationResult<Vec<Bytes>> {
        let payload = event.payload_with_type()?;
        if !is_anthropic_stream_event(&payload) {
            return Ok(Vec::new());
        }
        let parsed = serde_json::from_value::<MessageStreamEvent>(payload)?;
        let mut chunks = Vec::new();

        match parsed {
            MessageStreamEvent::MessageStart(event) => {
                self.response_id = Some(response_id(&event.message.id));
                self.model = Some(event.message.model.clone());
                self.input_tokens = event.message.usage.input_tokens;
                self.output_tokens = event.message.usage.output_tokens;
                let sequence_number = self.next_sequence_number();
                let response = self.response_snapshot("in_progress");
                chunks.push(self.openai_event(
                    "response.created",
                    json!({
                        "type": "response.created",
                        "sequence_number": sequence_number,
                        "response": response
                    }),
                )?);
            }
            MessageStreamEvent::ContentBlockStart(event) => {
                let item_id = format!("msg_{}", self.response_id());
                let index = event.index;
                let block = match event.content_block {
                    ContentBlock::Text(block) => {
                        let content_index = index;
                        let sequence_number = self.next_sequence_number();
                        chunks.push(self.openai_event(
                            "response.output_item.added",
                            json!({
                                "type": "response.output_item.added",
                                "sequence_number": sequence_number,
                                "output_index": index,
                                "item": {
                                    "type": "message",
                                    "id": item_id,
                                    "role": "assistant",
                                    "status": "in_progress",
                                    "content": []
                                }
                            }),
                        )?);
                        if !block.text.is_empty() {
                            let sequence_number = self.next_sequence_number();
                            chunks.push(self.openai_event(
                                "response.output_text.delta",
                                json!({
                                    "type": "response.output_text.delta",
                                    "sequence_number": sequence_number,
                                    "item_id": item_id,
                                    "output_index": index,
                                    "content_index": content_index,
                                    "delta": block.text
                                }),
                            )?);
                        }
                        AnthropicStreamBlock::Text {
                            item_id,
                            content_index,
                            text: block.text,
                        }
                    }
                    ContentBlock::Thinking(block) => {
                        let item_id = format!("rs_{}", self.response_id());
                        if !block.thinking.is_empty() {
                            let sequence_number = self.next_sequence_number();
                            chunks.push(self.openai_event(
                                "response.reasoning_summary_text.delta",
                                json!({
                                    "type": "response.reasoning_summary_text.delta",
                                    "sequence_number": sequence_number,
                                    "item_id": item_id,
                                    "output_index": index,
                                    "summary_index": 0,
                                    "delta": block.thinking
                                }),
                            )?);
                        }
                        AnthropicStreamBlock::Thinking {
                            item_id,
                            summary_index: 0,
                            text: block.thinking,
                        }
                    }
                    ContentBlock::ToolUse(block) => {
                        let sequence_number = self.next_sequence_number();
                        chunks.push(self.openai_event(
                            "response.output_item.added",
                            json!({
                                "type": "response.output_item.added",
                                "sequence_number": sequence_number,
                                "output_index": index,
                                "item": {
                                    "type": "function_call",
                                    "id": block.id,
                                    "call_id": block.id,
                                    "name": block.name,
                                    "arguments": "",
                                    "status": "in_progress"
                                }
                            }),
                        )?);
                        AnthropicStreamBlock::ToolUse {
                            item_id: block.id,
                            name: block.name,
                            arguments: String::new(),
                        }
                    }
                    _ => AnthropicStreamBlock::Other,
                };
                self.blocks.insert(index, block);
            }
            MessageStreamEvent::ContentBlockDelta(event) => {
                let index = event.index;
                let delta = event.delta;
                if !self.blocks.contains_key(&index) {
                    match &delta {
                        ContentBlockDelta::TextDelta(_) => {
                            self.ensure_text_block(index, &mut chunks)?;
                        }
                        ContentBlockDelta::ThinkingDelta(_) => {
                            self.ensure_thinking_block(index);
                        }
                        ContentBlockDelta::InputJsonDelta(_) => {
                            self.ensure_tool_block(index, None, None, &mut chunks)?;
                        }
                        _ => {}
                    }
                }
                let translated = match (self.blocks.get_mut(&index), delta) {
                    (
                        Some(AnthropicStreamBlock::Text {
                            item_id,
                            content_index,
                            text,
                        }),
                        ContentBlockDelta::TextDelta(delta),
                    ) => {
                        text.push_str(&delta.text);
                        Some((
                            "response.output_text.delta",
                            json!({
                                "type": "response.output_text.delta",
                                "item_id": item_id,
                                "output_index": index,
                                "content_index": *content_index,
                                "delta": delta.text
                            }),
                        ))
                    }
                    (
                        Some(AnthropicStreamBlock::Thinking {
                            item_id,
                            summary_index,
                            text,
                        }),
                        ContentBlockDelta::ThinkingDelta(delta),
                    ) => {
                        text.push_str(&delta.thinking);
                        Some((
                            "response.reasoning_summary_text.delta",
                            json!({
                                "type": "response.reasoning_summary_text.delta",
                                "item_id": item_id,
                                "output_index": index,
                                "summary_index": *summary_index,
                                "delta": delta.thinking
                            }),
                        ))
                    }
                    (
                        Some(AnthropicStreamBlock::ToolUse {
                            item_id, arguments, ..
                        }),
                        ContentBlockDelta::InputJsonDelta(delta),
                    ) => {
                        arguments.push_str(&delta.partial_json);
                        Some((
                            "response.function_call_arguments.delta",
                            json!({
                                "type": "response.function_call_arguments.delta",
                                "item_id": item_id,
                                "output_index": index,
                                "delta": delta.partial_json
                            }),
                        ))
                    }
                    _ => None,
                };
                if let Some((event_type, mut payload)) = translated {
                    payload["sequence_number"] = json!(self.next_sequence_number());
                    chunks.push(self.openai_event(event_type, payload)?);
                }
            }
            MessageStreamEvent::ContentBlockStop(event) => {
                let index = event.index;
                let Some(block) = self.blocks.get(&index).cloned() else {
                    return Ok(chunks);
                };
                match block {
                    AnthropicStreamBlock::Text {
                        item_id,
                        content_index,
                        text,
                    } => {
                        let sequence_number = self.next_sequence_number();
                        chunks.push(self.openai_event(
                            "response.output_text.done",
                            json!({
                                "type": "response.output_text.done",
                                "sequence_number": sequence_number,
                                "item_id": item_id,
                                "output_index": index,
                                "content_index": content_index,
                                "text": text
                            }),
                        )?);
                    }
                    AnthropicStreamBlock::Thinking {
                        item_id,
                        summary_index,
                        text,
                    } => {
                        let sequence_number = self.next_sequence_number();
                        chunks.push(self.openai_event(
                            "response.reasoning_summary_text.done",
                            json!({
                                "type": "response.reasoning_summary_text.done",
                                "sequence_number": sequence_number,
                                "item_id": item_id,
                                "output_index": index,
                                "summary_index": summary_index,
                                "text": text
                            }),
                        )?);
                    }
                    AnthropicStreamBlock::ToolUse {
                        item_id,
                        name,
                        arguments,
                    } => {
                        let sequence_number = self.next_sequence_number();
                        chunks.push(self.openai_event(
                            "response.function_call_arguments.done",
                            json!({
                                "type": "response.function_call_arguments.done",
                                "sequence_number": sequence_number,
                                "item_id": item_id,
                                "output_index": index,
                                "name": name,
                                "arguments": arguments
                            }),
                        )?);
                    }
                    AnthropicStreamBlock::Other => {}
                }
            }
            MessageStreamEvent::MessageDelta(event) => {
                if let Some(input_tokens) = event.usage.input_tokens {
                    self.input_tokens = input_tokens;
                }
                self.output_tokens = event.usage.output_tokens;
            }
            MessageStreamEvent::MessageStop(_) => {
                self.completed = true;
                let sequence_number = self.next_sequence_number();
                let response = self.response_snapshot("completed");
                chunks.push(self.openai_event(
                    "response.completed",
                    json!({
                        "type": "response.completed",
                        "sequence_number": sequence_number,
                        "response": response
                    }),
                )?);
            }
            MessageStreamEvent::Ping(_) => {}
        }

        Ok(chunks)
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

impl AnthropicToOpenaiStreamTranslator {
    fn next_sequence_number(&mut self) -> u64 {
        self.sequence_number += 1;
        self.sequence_number
    }

    fn response_id(&self) -> String {
        self.response_id
            .clone()
            .unwrap_or_else(|| "resp_stream".to_string())
    }

    fn ensure_text_block(
        &mut self,
        index: u32,
        chunks: &mut Vec<Bytes>,
    ) -> SseTranslationResult<()> {
        if self.blocks.contains_key(&index) {
            return Ok(());
        }
        let item_id = format!("msg_{}", self.response_id());
        let sequence_number = self.next_sequence_number();
        chunks.push(self.openai_event(
            "response.output_item.added",
            json!({
                "type": "response.output_item.added",
                "sequence_number": sequence_number,
                "output_index": index,
                "item": {
                    "type": "message",
                    "id": item_id,
                    "role": "assistant",
                    "status": "in_progress",
                    "content": []
                }
            }),
        )?);
        self.blocks.insert(
            index,
            AnthropicStreamBlock::Text {
                item_id,
                content_index: index,
                text: String::new(),
            },
        );
        Ok(())
    }

    fn ensure_thinking_block(&mut self, index: u32) {
        if self.blocks.contains_key(&index) {
            return;
        }
        self.blocks.insert(
            index,
            AnthropicStreamBlock::Thinking {
                item_id: format!("rs_{}", self.response_id()),
                summary_index: 0,
                text: String::new(),
            },
        );
    }

    fn ensure_tool_block(
        &mut self,
        index: u32,
        item_id: Option<String>,
        name: Option<String>,
        chunks: &mut Vec<Bytes>,
    ) -> SseTranslationResult<()> {
        if self.blocks.contains_key(&index) {
            return Ok(());
        }
        let id = item_id.unwrap_or_else(|| format!("toolu_{index}"));
        let name = name.unwrap_or_else(|| "function".to_string());
        let sequence_number = self.next_sequence_number();
        chunks.push(self.openai_event(
            "response.output_item.added",
            json!({
                "type": "response.output_item.added",
                "sequence_number": sequence_number,
                "output_index": index,
                "item": {
                    "type": "function_call",
                    "id": id,
                    "call_id": id,
                    "name": name,
                    "arguments": "",
                    "status": "in_progress"
                }
            }),
        )?);
        self.blocks.insert(
            index,
            AnthropicStreamBlock::ToolUse {
                item_id: id,
                name,
                arguments: String::new(),
            },
        );
        Ok(())
    }

    fn response_snapshot(&self, status: &str) -> Value {
        json!({
            "id": self.response_id(),
            "object": "response",
            "created_at": 0,
            "model": self.model.as_deref().unwrap_or_default(),
            "status": status,
            "output": [],
            "usage": {
                "input_tokens": self.input_tokens,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens": self.output_tokens,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": self.input_tokens.saturating_add(self.output_tokens)
            }
        })
    }

    fn openai_event<T>(&self, event_type: &str, payload: T) -> SseTranslationResult<Bytes>
    where
        T: Serialize,
    {
        Ok(encode_sse_json(event_type, &payload)?)
    }
}

fn translate_message(message: &Message) -> TranslationResult<Response> {
    Ok(Response {
        background: None,
        billing: None,
        conversation: None,
        created_at: 0,
        completed_at: None,
        error: None,
        id: response_id(&message.id),
        incomplete_details: None,
        instructions: None,
        max_output_tokens: None,
        metadata: None,
        model: message.model.clone(),
        object: "response".to_string(),
        output: translate_output(message)?,
        parallel_tool_calls: None,
        previous_response_id: None,
        prompt: None,
        prompt_cache_key: None,
        prompt_cache_retention: None,
        reasoning: None,
        safety_identifier: None,
        service_tier: None,
        status: response_status(message.stop_reason),
        temperature: None,
        text: None,
        tool_choice: None,
        tools: None,
        top_logprobs: None,
        top_p: None,
        truncation: None,
        usage: Some(ResponseUsage {
            input_tokens: message.usage.input_tokens,
            input_tokens_details: InputTokenDetails {
                cached_tokens: message.usage.cache_read_input_tokens.unwrap_or_default(),
            },
            output_tokens: message.usage.output_tokens,
            output_tokens_details: OutputTokenDetails {
                reasoning_tokens: 0,
            },
            total_tokens: message
                .usage
                .input_tokens
                .saturating_add(message.usage.output_tokens),
        }),
    })
}

fn translate_output(message: &Message) -> TranslationResult<Vec<OutputItem>> {
    let mut output = Vec::new();
    let mut text_content = Vec::new();

    for block in &message.content {
        match block {
            ContentBlock::Text(block) => {
                text_content.push(OutputMessageContent::OutputText(OutputTextContent {
                    text: block.text.clone(),
                    annotations: Vec::new(),
                    logprobs: None,
                }));
            }
            ContentBlock::Thinking(block) => output.push(OutputItem::Reasoning(ReasoningItem {
                id: Some(format!("rs_{}", message.id)),
                summary: vec![SummaryPart::SummaryText(SummaryTextContent {
                    text: block.thinking.clone(),
                })],
                encrypted_content: None,
                content: None,
                status: Some(OutputStatus::Completed),
            })),
            ContentBlock::RedactedThinking(block) => {
                output.push(OutputItem::Reasoning(ReasoningItem {
                    id: Some(format!("rs_{}", message.id)),
                    summary: Vec::new(),
                    encrypted_content: Some(block.data.clone()),
                    content: None,
                    status: Some(OutputStatus::Completed),
                }))
            }
            ContentBlock::ToolUse(block) => {
                output.push(OutputItem::FunctionCall(FunctionToolCall {
                    id: Some(block.id.clone()),
                    call_id: block.id.clone(),
                    name: block.name.clone(),
                    arguments: serde_json::to_string(&block.input)?,
                    status: Some(OutputStatus::Completed),
                    namespace: None,
                }))
            }
            other => text_content.push(OutputMessageContent::OutputText(OutputTextContent {
                text: format!(
                    "[Anthropic content block omitted during OpenAI Responses translation: {}]",
                    block_kind(other)
                ),
                annotations: Vec::new(),
                logprobs: None,
            })),
        }
    }

    if !text_content.is_empty() {
        output.insert(
            0,
            OutputItem::Message(OutputMessage {
                id: format!("msg_{}", message.id),
                role: AssistantRole::Assistant,
                status: OutputStatus::Completed,
                content: text_content,
                phase: None,
            }),
        );
    }

    Ok(output)
}

fn response_status(stop_reason: Option<StopReason>) -> Status {
    match stop_reason {
        Some(StopReason::MaxTokens) => Status::Incomplete,
        Some(StopReason::PauseTurn) | None => Status::InProgress,
        _ => Status::Completed,
    }
}

fn response_id(message_id: &str) -> String {
    if message_id.starts_with("resp_") {
        message_id.to_string()
    } else {
        format!("resp_{message_id}")
    }
}

fn block_kind(block: &ContentBlock) -> &'static str {
    match block {
        ContentBlock::Text(_) => "text",
        ContentBlock::Thinking(_) => "thinking",
        ContentBlock::RedactedThinking(_) => "redacted_thinking",
        ContentBlock::ToolUse(_) => "tool_use",
        ContentBlock::ServerToolUse(_) => "server_tool_use",
        ContentBlock::WebSearchToolResult(_) => "web_search_tool_result",
        ContentBlock::WebFetchToolResult(_) => "web_fetch_tool_result",
        ContentBlock::CodeExecutionToolResult(_) => "code_execution_tool_result",
        ContentBlock::BashCodeExecutionToolResult(_) => "bash_code_execution_tool_result",
        ContentBlock::TextEditorCodeExecutionToolResult(_) => {
            "text_editor_code_execution_tool_result"
        }
        ContentBlock::ToolSearchToolResult(_) => "tool_search_tool_result",
        ContentBlock::ContainerUpload(_) => "container_upload",
    }
}

#[cfg(test)]
mod tests;
