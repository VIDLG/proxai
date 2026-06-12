//! `openai_chat_completions -> openai_responses` response translation.

mod types;

use axum::body::Bytes;
use serde_json::{Value, json};
use std::collections::BTreeMap;

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCalls, CreateChatCompletionResponse, FinishReason,
};
use crate::protocol::openai_responses::{
    AssistantRole, CustomToolCall, FunctionToolCall, InputTokenDetails, OutputItem, OutputMessage,
    OutputMessageContent, OutputStatus, OutputTextContent, OutputTokenDetails, RefusalContent,
    Response, ResponseUsage, Status,
};
use crate::sse::{SseEvent, done_sentinel_bytes};
use crate::translation::TranslationResult;

use crate::http_support::ByteStream;
use crate::translation::streaming::{
    SseStreamEnd, StreamTranslationResult, StreamingEventTranslator, encode_sse_json,
    translate_sse_stream,
};

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, ChatToResponsesStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(payload)?;
    let translated = translate_chat_response(&chat);
    Ok(serde_json::to_value(translated)?)
}

fn translate_chat_response(chat: &CreateChatCompletionResponse) -> Response {
    let mut output = Vec::new();
    for choice in &chat.choices {
        let message = &choice.message;
        if let Some(tool_calls) = message.tool_calls.as_ref() {
            for tool_call in tool_calls {
                output.push(translate_tool_call(tool_call));
            }
        }
        if message
            .content
            .as_ref()
            .is_some_and(|value| !value.is_empty())
            || message
                .refusal
                .as_ref()
                .is_some_and(|value| !value.is_empty())
        {
            let mut content = Vec::new();
            if let Some(text) = message.content.as_ref().filter(|value| !value.is_empty()) {
                content.push(OutputMessageContent::OutputText(OutputTextContent {
                    text: text.clone(),
                    annotations: Vec::new(),
                    logprobs: None,
                }));
            }
            if let Some(refusal) = message.refusal.as_ref().filter(|value| !value.is_empty()) {
                content.push(OutputMessageContent::Refusal(RefusalContent {
                    refusal: refusal.clone(),
                }));
            }
            output.push(OutputItem::Message(OutputMessage {
                id: format!("msg_{}_{}", chat.id, choice.index),
                role: AssistantRole::Assistant,
                status: OutputStatus::Completed,
                content,
                phase: None,
            }));
        }
    }

    if output.is_empty() {
        output.push(OutputItem::Message(OutputMessage {
            id: format!("msg_{}", chat.id),
            role: AssistantRole::Assistant,
            status: OutputStatus::Completed,
            content: Vec::new(),
            phase: None,
        }));
    }

    let usage = chat.usage.as_ref();
    let input_tokens = usage.map(|usage| usage.prompt_tokens).unwrap_or_default();
    let output_tokens = usage
        .map(|usage| usage.completion_tokens)
        .unwrap_or_default();
    let total_tokens = usage
        .map(|usage| usage.total_tokens)
        .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

    Response {
        background: None,
        billing: None,
        conversation: None,
        created_at: chat.created as u64,
        completed_at: None,
        error: None,
        id: response_id(&chat.id),
        incomplete_details: None,
        instructions: None,
        max_output_tokens: None,
        metadata: None,
        model: chat.model.clone(),
        object: "response".to_string(),
        output,
        parallel_tool_calls: None,
        previous_response_id: None,
        prompt: None,
        prompt_cache_key: None,
        prompt_cache_retention: None,
        reasoning: None,
        safety_identifier: None,
        service_tier: None,
        status: response_status(chat),
        temperature: None,
        text: None,
        tool_choice: None,
        tools: None,
        top_logprobs: None,
        top_p: None,
        truncation: None,
        usage: Some(ResponseUsage {
            input_tokens,
            input_tokens_details: InputTokenDetails { cached_tokens: 0 },
            output_tokens,
            output_tokens_details: OutputTokenDetails {
                reasoning_tokens: usage
                    .and_then(|usage| usage.completion_tokens_details)
                    .and_then(|details| details.reasoning_tokens)
                    .unwrap_or_default(),
            },
            total_tokens,
        }),
    }
}

fn translate_tool_call(tool_call: &ChatCompletionMessageToolCalls) -> OutputItem {
    match tool_call {
        ChatCompletionMessageToolCalls::Function(call) => {
            OutputItem::FunctionCall(FunctionToolCall {
                id: Some(call.id.clone()),
                call_id: call.id.clone(),
                name: call.function.name.clone(),
                arguments: call.function.arguments.clone(),
                status: Some(OutputStatus::Completed),
                namespace: None,
            })
        }
        ChatCompletionMessageToolCalls::Custom(call) => {
            OutputItem::CustomToolCall(CustomToolCall {
                id: Some(call.id.clone()),
                call_id: call.id.clone(),
                name: call.custom_tool.name.clone(),
                input: call.custom_tool.input.clone(),
                namespace: None,
            })
        }
    }
}

fn response_status(chat: &CreateChatCompletionResponse) -> Status {
    if chat.choices.iter().any(|choice| {
        matches!(
            choice.finish_reason,
            Some(FinishReason::Length | FinishReason::ContentFilter)
        )
    }) {
        Status::Incomplete
    } else {
        Status::Completed
    }
}

#[derive(Debug, Default)]
struct ChatToResponsesStreamTranslator {
    sequence_number: u64,
    response_id: Option<String>,
    model: Option<String>,
    created_at: Option<u64>,
    text_items: BTreeMap<u32, StreamTextItem>,
    tool_items: BTreeMap<u32, StreamToolItem>,
    usage: Option<Value>,
    completed: bool,
}

#[derive(Debug, Clone)]
struct StreamTextItem {
    item_id: String,
    text: String,
}

#[derive(Debug, Clone)]
struct StreamToolItem {
    item_id: String,
    name: String,
    arguments: String,
}

impl StreamingEventTranslator for ChatToResponsesStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> StreamTranslationResult<Vec<Bytes>> {
        let payload = event.payload_with_type()?;
        let mut chunks = Vec::new();

        if self.response_id.is_none() {
            if let Some(id) = payload.get("id").and_then(Value::as_str) {
                self.response_id = Some(response_id(id));
            }
            if let Some(model) = payload.get("model").and_then(Value::as_str) {
                self.model = Some(model.to_string());
            }
            self.created_at = payload.get("created").and_then(Value::as_u64);
            let sequence_number = self.next_sequence_number();
            chunks.push(self.responses_event(
                "response.created",
                json!({
                    "type": "response.created",
                    "sequence_number": sequence_number,
                    "response": self.response_snapshot("in_progress")
                }),
            )?);
        }

        if let Some(usage) = payload.get("usage").filter(|value| !value.is_null()) {
            self.usage = Some(usage.clone());
        }

        for choice in payload
            .get("choices")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let index = choice.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
            let Some(delta) = choice.get("delta").and_then(Value::as_object) else {
                continue;
            };

            if let Some(content) = delta.get("content").and_then(Value::as_str) {
                self.ensure_text_item(index, &mut chunks)?;
                if let Some(item) = self.text_items.get_mut(&index) {
                    item.text.push_str(content);
                    let item_id = item.item_id.clone();
                    let sequence_number = self.next_sequence_number();
                    chunks.push(self.responses_event(
                        "response.output_text.delta",
                        json!({
                            "type": "response.output_text.delta",
                            "sequence_number": sequence_number,
                            "item_id": item_id,
                            "output_index": index,
                            "content_index": 0,
                            "delta": content
                        }),
                    )?);
                }
            }

            if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for tool_call in tool_calls {
                    let tool_index = tool_call
                        .get("index")
                        .and_then(Value::as_u64)
                        .unwrap_or(index as u64) as u32;
                    self.ensure_tool_item(tool_index, tool_call, &mut chunks)?;
                    if let Some(function) = tool_call.get("function").and_then(Value::as_object) {
                        if let Some(name) = function.get("name").and_then(Value::as_str)
                            && let Some(item) = self.tool_items.get_mut(&tool_index)
                        {
                            item.name = name.to_string();
                        }
                        if let Some(arguments) = function.get("arguments").and_then(Value::as_str)
                            && let Some(item) = self.tool_items.get_mut(&tool_index)
                        {
                            item.arguments.push_str(arguments);
                            let item_id = item.item_id.clone();
                            let sequence_number = self.next_sequence_number();
                            chunks.push(self.responses_event(
                                "response.function_call_arguments.delta",
                                json!({
                                    "type": "response.function_call_arguments.delta",
                                    "sequence_number": sequence_number,
                                    "item_id": item_id,
                                    "output_index": tool_index,
                                    "delta": arguments
                                }),
                            )?);
                        }
                    }
                }
            }
        }

        Ok(chunks)
    }

    fn finish_stream(&mut self, _end: SseStreamEnd) -> StreamTranslationResult<Vec<Bytes>> {
        if self.completed {
            return Ok(Vec::new());
        }
        self.completed = true;
        let mut chunks = Vec::new();

        for (index, item) in self.text_items.clone() {
            let sequence_number = self.next_sequence_number();
            chunks.push(self.responses_event(
                "response.output_text.done",
                json!({
                    "type": "response.output_text.done",
                    "sequence_number": sequence_number,
                    "item_id": item.item_id,
                    "output_index": index,
                    "content_index": 0,
                    "text": item.text
                }),
            )?);
        }
        for (index, item) in self.tool_items.clone() {
            let sequence_number = self.next_sequence_number();
            chunks.push(self.responses_event(
                "response.function_call_arguments.done",
                json!({
                    "type": "response.function_call_arguments.done",
                    "sequence_number": sequence_number,
                    "item_id": item.item_id,
                    "output_index": index,
                    "name": item.name,
                    "arguments": item.arguments
                }),
            )?);
        }
        let sequence_number = self.next_sequence_number();
        chunks.push(self.responses_event(
            "response.completed",
            json!({
                "type": "response.completed",
                "sequence_number": sequence_number,
                "response": self.response_snapshot("completed")
            }),
        )?);
        chunks.push(done_sentinel_bytes());
        Ok(chunks)
    }
}

impl ChatToResponsesStreamTranslator {
    fn next_sequence_number(&mut self) -> u64 {
        let sequence_number = self.sequence_number;
        self.sequence_number += 1;
        sequence_number
    }

    fn response_id(&self) -> String {
        self.response_id
            .clone()
            .unwrap_or_else(|| "resp_chatcmpl".to_string())
    }

    fn model(&self) -> String {
        self.model.clone().unwrap_or_default()
    }

    fn response_snapshot(&self, status: &str) -> Value {
        let usage = self.usage.as_ref();
        let input_tokens = usage
            .and_then(|usage| usage.get("prompt_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let output_tokens = usage
            .and_then(|usage| usage.get("completion_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let total_tokens = usage
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or_else(|| input_tokens.saturating_add(output_tokens));

        json!({
            "id": self.response_id(),
            "object": "response",
            "created_at": self.created_at.unwrap_or_default(),
            "model": self.model(),
            "output": [],
            "status": status,
            "usage": {
                "input_tokens": input_tokens,
                "input_tokens_details": {"cached_tokens": 0},
                "output_tokens": output_tokens,
                "output_tokens_details": {"reasoning_tokens": 0},
                "total_tokens": total_tokens
            }
        })
    }

    fn ensure_text_item(
        &mut self,
        index: u32,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.text_items.contains_key(&index) {
            return Ok(());
        }
        let item_id = format!("msg_{}_{index}", self.response_id());
        let sequence_number = self.next_sequence_number();
        chunks.push(self.responses_event(
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
        self.text_items.insert(
            index,
            StreamTextItem {
                item_id,
                text: String::new(),
            },
        );
        Ok(())
    }

    fn ensure_tool_item(
        &mut self,
        index: u32,
        tool_call: &Value,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.tool_items.contains_key(&index) {
            return Ok(());
        }
        let item_id = tool_call
            .get("id")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("call_{}_{index}", self.response_id()));
        let name = tool_call
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
            .unwrap_or("function")
            .to_string();
        let sequence_number = self.next_sequence_number();
        chunks.push(self.responses_event(
            "response.output_item.added",
            json!({
                "type": "response.output_item.added",
                "sequence_number": sequence_number,
                "output_index": index,
                "item": {
                    "type": "function_call",
                    "id": item_id,
                    "call_id": item_id,
                    "name": name,
                    "arguments": "",
                    "status": "in_progress"
                }
            }),
        )?);
        self.tool_items.insert(
            index,
            StreamToolItem {
                item_id,
                name,
                arguments: String::new(),
            },
        );
        Ok(())
    }

    fn responses_event(&self, event: &str, payload: Value) -> StreamTranslationResult<Bytes> {
        Ok(encode_sse_json(event, &payload)?)
    }
}

fn response_id(chat_id: &str) -> String {
    if chat_id.starts_with("resp_") {
        chat_id.to_string()
    } else {
        format!("resp_{chat_id}")
    }
}

#[cfg(test)]
mod tests;
