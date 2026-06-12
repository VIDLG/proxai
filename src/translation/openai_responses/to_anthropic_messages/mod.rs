//! `openai_responses -> anthropic_messages` request translation.
//!
//! This is intentionally request-focused. Response translation is not handled here;
//! the selected provider protocol owns upstream response handling.

mod types;

use axum::body::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::anthropic::messages::{
    ContentBlock, DirectCaller, Message as AnthropicMessage, MessageCreateParamsBase, MessageType,
    Role, StopReason, TextBlock, ThinkingBlock, ToolCaller, ToolUseBlock, Usage,
};
use crate::translation::{TranslationError, TranslationResult};

use crate::sse::SseEvent;

use crate::http_support::ByteStream;
use crate::translation::streaming::{
    StreamTranslationResult, StreamingEventTranslator, encode_sse_json, translate_sse_stream,
};

const DEFAULT_MAX_TOKENS: u32 = 4096;

#[derive(Debug, Default, Deserialize)]
struct ResponseRequestView {
    model: String,
    input: Option<Value>,
    instructions: Option<String>,
    max_output_tokens: Option<u32>,
    metadata: Option<Value>,
    parallel_tool_calls: Option<bool>,
    reasoning: Option<Value>,
    stream: Option<bool>,
    temperature: Option<f32>,
    tool_choice: Option<Value>,
    tools: Option<Value>,
    top_p: Option<f32>,
}

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let source = serde_json::from_value::<ResponseRequestView>(payload.clone())?;
    let input = source.input.as_ref();
    let tools = source.tools.as_ref();
    let tool_choice = source.tool_choice.as_ref();
    let reasoning = source.reasoning.as_ref();
    let metadata = source.metadata.as_ref();

    let mut system_parts = Vec::new();
    if let Some(instructions) = source.instructions.as_deref()
        && !instructions.trim().is_empty()
    {
        system_parts.push(instructions.to_string());
    }

    let mut messages = translate_input(input, &mut system_parts)?;
    if messages.is_empty() {
        messages.push(json!({
            "role": "user",
            "content": ""
        }));
    }

    let mut request = Map::new();
    request.insert("model".to_string(), Value::String(source.model));
    request.insert(
        "max_tokens".to_string(),
        json!(source.max_output_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
    );
    request.insert("messages".to_string(), Value::Array(messages));

    if let Some(system) = join_text_parts(system_parts) {
        request.insert("system".to_string(), Value::String(system));
    }
    if let Some(stream) = source.stream {
        request.insert("stream".to_string(), Value::Bool(stream));
    }
    if let Some(temperature) = source.temperature {
        request.insert("temperature".to_string(), json!(temperature));
    }
    if let Some(top_p) = source.top_p {
        request.insert("top_p".to_string(), json!(top_p));
    }

    if let Some(metadata) = translate_metadata(metadata) {
        request.insert("metadata".to_string(), metadata);
    }
    if let Some(thinking) = translate_reasoning(reasoning) {
        request.insert("thinking".to_string(), thinking);
    }
    if let Some(tools) = translate_tools(tools) {
        request.insert("tools".to_string(), tools);
    }
    if let Some(tool_choice) = translate_tool_choice(tool_choice, source.parallel_tool_calls) {
        request.insert("tool_choice".to_string(), tool_choice);
    }

    let typed = serde_json::from_value::<MessageCreateParamsBase>(Value::Object(request))?;
    Ok(serde_json::to_value(typed)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, OpenaiToAnthropicStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let value = serde_json::from_value::<OpenaiResponseBody>(payload)?;
    let translated = translate_response_payload(&value);
    Ok(serde_json::to_value(translated)?)
}

#[derive(Debug, Default)]
struct OpenaiToAnthropicStreamTranslator {
    message_started: bool,
    message_id: Option<String>,
    model: Option<String>,
    input_tokens: u32,
    output_tokens: u32,
    blocks: BTreeMap<u32, OpenaiStreamBlock>,
    stopped_blocks: BTreeSet<u32>,
    completed: bool,
}

#[derive(Debug, Clone)]
enum OpenaiStreamBlock {
    Text,
    Thinking,
    ToolUse,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenaiStreamEvent {
    #[serde(rename = "response.created")]
    ResponseCreated { response: OpenaiStreamResponse },
    #[serde(rename = "response.in_progress")]
    ResponseInProgress { response: OpenaiStreamResponse },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        output_index: u32,
        item: OpenaiStreamOutputItem,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        output_index: Option<u32>,
        delta: String,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone { output_index: Option<u32> },
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta {
        output_index: Option<u32>,
        delta: String,
    },
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone { output_index: Option<u32> },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        output_index: Option<u32>,
        item_id: Option<String>,
        delta: String,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        output_index: Option<u32>,
        item_id: Option<String>,
        name: Option<String>,
    },
    #[serde(rename = "response.completed")]
    ResponseCompleted {
        response: Option<OpenaiStreamResponse>,
    },
    #[serde(rename = "response.incomplete")]
    ResponseIncomplete {
        response: Option<OpenaiStreamResponse>,
    },
    #[serde(rename = "response.failed")]
    ResponseFailed {
        response: Option<OpenaiStreamResponse>,
    },
    #[serde(rename = "response.error")]
    ResponseError,
}

#[derive(Debug, Deserialize)]
struct OpenaiStreamResponse {
    id: Option<String>,
    model: Option<String>,
    usage: Option<OpenaiResponseUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenaiStreamOutputItem {
    #[serde(rename = "message")]
    Message { id: Option<String> },
    #[serde(rename = "function_call")]
    FunctionCall {
        id: Option<String>,
        call_id: Option<String>,
        name: Option<String>,
    },
    #[serde(rename = "reasoning")]
    Reasoning { id: Option<String> },
}

impl StreamingEventTranslator for OpenaiToAnthropicStreamTranslator {
    fn translate_event(&mut self, event: SseEvent) -> StreamTranslationResult<Vec<Bytes>> {
        let parsed = serde_json::from_value::<OpenaiStreamEvent>(event.payload_with_type()?)?;
        let mut chunks = Vec::new();

        match parsed {
            OpenaiStreamEvent::ResponseCreated { response }
            | OpenaiStreamEvent::ResponseInProgress { response } => {
                self.record_response(response);
                if !self.message_started {
                    self.message_started = true;
                    chunks.push(self.anthropic_event(
                        "message_start",
                        json!({
                            "type": "message_start",
                            "message": {
                                "id": self.message_id(),
                                "type": "message",
                                "role": "assistant",
                                "model": self.model.as_deref().unwrap_or_default(),
                                "content": [],
                                "stop_reason": null,
                                "stop_sequence": null,
                                "stop_details": null,
                                "container": null,
                                "usage": self.message_usage(0)
                            }
                        }),
                    )?);
                }
            }
            OpenaiStreamEvent::OutputItemAdded { output_index, item } => {
                let block = match item {
                    OpenaiStreamOutputItem::Message { id } => {
                        let _ = id;
                        chunks.push(self.anthropic_event(
                            "content_block_start",
                            json!({
                                "type": "content_block_start",
                                "index": output_index,
                                "content_block": {"type": "text", "text": "", "citations": null}
                            }),
                        )?);
                        OpenaiStreamBlock::Text
                    }
                    OpenaiStreamOutputItem::Reasoning { id } => {
                        let _ = id;
                        chunks.push(self.anthropic_event(
                            "content_block_start",
                            json!({
                                "type": "content_block_start",
                                "index": output_index,
                                "content_block": {"type": "thinking", "thinking": "", "signature": ""}
                            }),
                        )?);
                        OpenaiStreamBlock::Thinking
                    }
                    OpenaiStreamOutputItem::FunctionCall { id, call_id, name } => {
                        let id = call_id
                            .or(id)
                            .unwrap_or_else(|| format!("toolu_{output_index}"));
                        let name = name.unwrap_or_else(|| "function".to_string());
                        chunks.push(self.anthropic_event(
                            "content_block_start",
                            json!({
                                "type": "content_block_start",
                                "index": output_index,
                                "content_block": {
                                    "type": "tool_use",
                                    "id": id,
                                    "caller": {"type": "direct"},
                                    "name": name,
                                    "input": {}
                                }
                            }),
                        )?);
                        let _ = (id, name);
                        OpenaiStreamBlock::ToolUse
                    }
                };
                self.blocks.insert(output_index, block);
            }
            OpenaiStreamEvent::OutputTextDelta {
                output_index,
                delta,
            } => {
                let index = output_index.unwrap_or(0);
                self.ensure_text_block(index, &mut chunks)?;
                chunks.push(self.anthropic_event(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "text_delta", "text": delta}
                    }),
                )?);
            }
            OpenaiStreamEvent::OutputTextDone { output_index } => {
                self.stop_block(output_index.unwrap_or(0), &mut chunks)?;
            }
            OpenaiStreamEvent::ReasoningSummaryTextDelta {
                output_index,
                delta,
            } => {
                let index = output_index.unwrap_or(0);
                self.ensure_thinking_block(index, &mut chunks)?;
                chunks.push(self.anthropic_event(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "thinking_delta", "thinking": delta}
                    }),
                )?);
            }
            OpenaiStreamEvent::ReasoningSummaryTextDone { output_index } => {
                self.stop_block(output_index.unwrap_or(0), &mut chunks)?;
            }
            OpenaiStreamEvent::FunctionCallArgumentsDelta {
                output_index,
                item_id,
                delta,
            } => {
                let index = output_index.unwrap_or(0);
                self.ensure_tool_block(index, item_id, None, &mut chunks)?;
                chunks.push(self.anthropic_event(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {"type": "input_json_delta", "partial_json": delta}
                    }),
                )?);
            }
            OpenaiStreamEvent::FunctionCallArgumentsDone {
                output_index,
                item_id,
                name,
            } => {
                let index = output_index.unwrap_or(0);
                self.ensure_tool_block(index, item_id, name, &mut chunks)?;
                self.stop_block(index, &mut chunks)?;
            }
            OpenaiStreamEvent::ResponseCompleted { response } => {
                if let Some(response) = response {
                    self.record_response(response);
                }
                self.complete(StopReason::EndTurn, &mut chunks)?;
            }
            OpenaiStreamEvent::ResponseIncomplete { response } => {
                if let Some(response) = response {
                    self.record_response(response);
                }
                self.complete(StopReason::MaxTokens, &mut chunks)?;
            }
            OpenaiStreamEvent::ResponseFailed { response } => {
                if let Some(response) = response {
                    self.record_response(response);
                }
                self.complete(StopReason::Refusal, &mut chunks)?;
            }
            OpenaiStreamEvent::ResponseError => {
                self.complete(StopReason::Refusal, &mut chunks)?;
            }
        }

        Ok(chunks)
    }
}

impl OpenaiToAnthropicStreamTranslator {
    fn record_response(&mut self, response: OpenaiStreamResponse) {
        if let Some(id) = response.id {
            self.message_id = Some(id);
        }
        if let Some(model) = response.model {
            self.model = Some(model);
        }
        if let Some(usage) = response.usage {
            self.input_tokens = usage.input_tokens;
            self.output_tokens = usage.output_tokens;
        }
    }

    fn ensure_text_block(
        &mut self,
        index: u32,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.blocks.contains_key(&index) {
            return Ok(());
        }
        chunks.push(self.anthropic_event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {"type": "text", "text": "", "citations": null}
            }),
        )?);
        self.blocks.insert(index, OpenaiStreamBlock::Text);
        Ok(())
    }

    fn ensure_thinking_block(
        &mut self,
        index: u32,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.blocks.contains_key(&index) {
            return Ok(());
        }
        chunks.push(self.anthropic_event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {"type": "thinking", "thinking": "", "signature": ""}
            }),
        )?);
        self.blocks.insert(index, OpenaiStreamBlock::Thinking);
        Ok(())
    }

    fn ensure_tool_block(
        &mut self,
        index: u32,
        item_id: Option<String>,
        name: Option<String>,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.blocks.contains_key(&index) {
            return Ok(());
        }
        let id = item_id.unwrap_or_else(|| format!("toolu_{index}"));
        let name = name.unwrap_or_else(|| "function".to_string());
        chunks.push(self.anthropic_event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {
                    "type": "tool_use",
                    "id": id,
                    "caller": {"type": "direct"},
                    "name": name,
                    "input": {}
                }
            }),
        )?);
        let _ = (id, name);
        self.blocks.insert(index, OpenaiStreamBlock::ToolUse);
        Ok(())
    }

    fn stop_block(&mut self, index: u32, chunks: &mut Vec<Bytes>) -> StreamTranslationResult<()> {
        if self.stopped_blocks.insert(index) {
            chunks.push(self.anthropic_event(
                "content_block_stop",
                json!({"type": "content_block_stop", "index": index}),
            )?);
        }
        Ok(())
    }

    fn complete(
        &mut self,
        stop_reason: StopReason,
        chunks: &mut Vec<Bytes>,
    ) -> StreamTranslationResult<()> {
        if self.completed {
            return Ok(());
        }
        self.completed = true;

        let open_indexes = self.blocks.keys().copied().collect::<Vec<_>>();
        for index in open_indexes {
            self.stop_block(index, chunks)?;
        }

        chunks.push(self.anthropic_event(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": stop_reason,
                    "stop_sequence": null,
                    "stop_details": null,
                    "container": null
                },
                "usage": {
                    "input_tokens": self.input_tokens,
                    "output_tokens": self.output_tokens,
                    "cache_creation_input_tokens": null,
                    "cache_read_input_tokens": null,
                    "server_tool_use": null
                }
            }),
        )?);
        chunks.push(self.anthropic_event("message_stop", json!({"type": "message_stop"}))?);
        Ok(())
    }

    fn message_id(&self) -> String {
        self.message_id
            .clone()
            .unwrap_or_else(|| "msg_response".to_string())
    }

    fn message_usage(&self, output_tokens: u32) -> Value {
        json!({
            "input_tokens": self.input_tokens,
            "output_tokens": output_tokens,
            "cache_creation": null,
            "cache_creation_input_tokens": null,
            "cache_read_input_tokens": null,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": null
        })
    }

    fn anthropic_event<T>(&self, event_type: &str, payload: T) -> StreamTranslationResult<Bytes>
    where
        T: Serialize,
    {
        Ok(encode_sse_json(event_type, &payload)?)
    }
}

#[derive(Debug, Deserialize)]
struct OpenaiResponseBody {
    id: String,
    model: String,
    status: Option<OpenaiResponseStatus>,
    #[serde(default)]
    output: Vec<OpenaiOutputItem>,
    usage: Option<OpenaiResponseUsage>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OpenaiResponseStatus {
    Completed,
    Failed,
    InProgress,
    Cancelled,
    Queued,
    Incomplete,
}

#[derive(Debug, Deserialize)]
struct OpenaiResponseUsage {
    input_tokens: u32,
    output_tokens: u32,
    input_tokens_details: Option<OpenaiInputTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenaiInputTokenDetails {
    cached_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenaiOutputItem {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        content: Vec<OpenaiMessageContent>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        id: Option<String>,
        call_id: Option<String>,
        name: String,
        arguments: Option<String>,
    },
    #[serde(rename = "custom_tool_call")]
    CustomToolCall {
        id: Option<String>,
        call_id: Option<String>,
        name: String,
        input: Option<Value>,
    },
    #[serde(rename = "reasoning")]
    Reasoning {
        #[serde(default)]
        summary: Vec<OpenaiReasoningSummaryPart>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenaiMessageContent {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum OpenaiReasoningSummaryPart {
    #[serde(rename = "summary_text")]
    SummaryText { text: String },
    #[serde(other)]
    Unknown,
}

fn translate_response_payload(response: &OpenaiResponseBody) -> AnthropicMessage {
    let content = response
        .output
        .iter()
        .flat_map(translate_response_output_item)
        .collect::<Vec<_>>();

    AnthropicMessage {
        id: response.id.clone(),
        container: None,
        content,
        model: response.model.clone(),
        role: Role::Assistant,
        type_: MessageType::Message,
        stop_details: None,
        stop_reason: anthropic_stop_reason(response.status),
        stop_sequence: None,
        usage: anthropic_usage(response.usage.as_ref()),
    }
}

fn translate_input(
    input: Option<&Value>,
    system_parts: &mut Vec<String>,
) -> TranslationResult<Vec<Value>> {
    let Some(input) = input else {
        return Ok(Vec::new());
    };

    match input {
        Value::String(text) => Ok(vec![message("user", Value::String(text.clone()))]),
        Value::Array(items) => {
            let mut messages = Vec::new();
            for item in items {
                translate_input_item(item, system_parts, &mut messages)?;
            }
            Ok(messages)
        }
        _ => Err(TranslationError::InvalidPayload(
            "openai_responses -> anthropic_messages translation requires `input` to be a string or array"
                .to_string(),
        )),
    }
}

fn translate_response_output_item(item: &OpenaiOutputItem) -> Vec<ContentBlock> {
    match item {
        OpenaiOutputItem::Message { content } => content
            .iter()
            .filter_map(translate_response_message_content)
            .collect(),
        OpenaiOutputItem::FunctionCall {
            id,
            call_id,
            name,
            arguments,
        } => vec![ContentBlock::ToolUse(ToolUseBlock {
            id: call_id
                .clone()
                .or_else(|| id.clone())
                .unwrap_or_else(|| "tool_call".to_string()),
            caller: ToolCaller::Direct(DirectCaller),
            input: arguments
                .as_deref()
                .and_then(|arguments| serde_json::from_str::<Value>(arguments).ok())
                .unwrap_or_else(|| json!({})),
            name: name.clone(),
        })],
        OpenaiOutputItem::CustomToolCall {
            id,
            call_id,
            name,
            input,
        } => vec![ContentBlock::ToolUse(ToolUseBlock {
            id: call_id
                .clone()
                .or_else(|| id.clone())
                .unwrap_or_else(|| "custom_tool_call".to_string()),
            caller: ToolCaller::Direct(DirectCaller),
            input: input
                .clone()
                .unwrap_or_else(|| Value::String(String::new())),
            name: name.clone(),
        })],
        OpenaiOutputItem::Reasoning { summary } => summary
            .iter()
            .filter_map(|part| match part {
                OpenaiReasoningSummaryPart::SummaryText { text } => {
                    Some(ContentBlock::Thinking(ThinkingBlock {
                        thinking: text.clone(),
                        signature: String::new(),
                    }))
                }
                OpenaiReasoningSummaryPart::Unknown => None,
            })
            .collect(),
        OpenaiOutputItem::Unknown => vec![ContentBlock::Text(TextBlock {
            citations: None,
            text: "[OpenAI Responses output item omitted during Anthropic translation]".to_string(),
        })],
    }
}

fn translate_response_message_content(content: &OpenaiMessageContent) -> Option<ContentBlock> {
    match content {
        OpenaiMessageContent::OutputText { text } => Some(ContentBlock::Text(TextBlock {
            citations: None,
            text: text.clone(),
        })),
        OpenaiMessageContent::Refusal { refusal } => Some(ContentBlock::Text(TextBlock {
            citations: None,
            text: refusal.clone(),
        })),
        OpenaiMessageContent::Unknown => None,
    }
}

fn anthropic_stop_reason(status: Option<OpenaiResponseStatus>) -> Option<StopReason> {
    match status {
        Some(OpenaiResponseStatus::Completed) => Some(StopReason::EndTurn),
        Some(OpenaiResponseStatus::Incomplete) => Some(StopReason::MaxTokens),
        Some(OpenaiResponseStatus::Failed) | Some(OpenaiResponseStatus::Cancelled) => {
            Some(StopReason::Refusal)
        }
        _ => None,
    }
}

fn anthropic_usage(usage: Option<&OpenaiResponseUsage>) -> Usage {
    Usage {
        cache_creation: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: usage
            .and_then(|usage| usage.input_tokens_details.as_ref())
            .map(|details| details.cached_tokens),
        inference_geo: None,
        input_tokens: usage.map(|usage| usage.input_tokens).unwrap_or_default(),
        output_tokens: usage.map(|usage| usage.output_tokens).unwrap_or_default(),
        output_tokens_details: None,
        server_tool_use: None,
        service_tier: None,
    }
}

fn translate_input_item(
    item: &Value,
    system_parts: &mut Vec<String>,
    messages: &mut Vec<Value>,
) -> TranslationResult<()> {
    let Some(object) = item.as_object() else {
        messages.push(message("user", Value::String(item.to_string())));
        return Ok(());
    };

    match object.get("type").and_then(Value::as_str) {
        Some("message") | None => translate_message_object(object, system_parts, messages),
        Some("function_call") => {
            // Anthropic Messages represents a tool roundtrip as an assistant
            // message containing one or more tool_use blocks followed by the
            // next user message containing the matching tool_result blocks.
            // MiniMax M3 strictly enforces this adjacency and rejects split or
            // interleaved tool turns with `tool call result does not follow tool
            // call (2013)`, so keep adjacent OpenAI tool calls in one assistant
            // message during translation.
            append_message_content_block(messages, "assistant", translate_function_call(object)?);
            Ok(())
        }
        Some("function_call_output") => {
            append_message_content_block(messages, "user", translate_function_call_output(object));
            Ok(())
        }
        Some("custom_tool_call") => {
            append_message_content_block(messages, "assistant", translate_custom_tool_call(object));
            Ok(())
        }
        Some("custom_tool_call_output") => {
            append_message_content_block(
                messages,
                "user",
                translate_custom_tool_call_output(object),
            );
            Ok(())
        }
        Some(kind) => {
            messages.push(message(
                "user",
                Value::String(format!(
                    "[OpenAI Responses item `{kind}` omitted during Anthropic translation]"
                )),
            ));
            Ok(())
        }
    }
}

fn translate_message_object(
    object: &Map<String, Value>,
    system_parts: &mut Vec<String>,
    messages: &mut Vec<Value>,
) -> TranslationResult<()> {
    let role = object.get("role").and_then(Value::as_str).unwrap_or("user");
    let empty_content = Value::String(String::new());
    let content = object.get("content").unwrap_or(&empty_content);

    if matches!(role, "system" | "developer") {
        if let Some(text) = extract_text(content) {
            system_parts.push(text);
        }
        return Ok(());
    }

    let anthropic_role = if role == "assistant" {
        "assistant"
    } else {
        "user"
    };
    messages.push(message(anthropic_role, translate_message_content(content)?));
    Ok(())
}

fn translate_message_content(content: &Value) -> TranslationResult<Value> {
    match content {
        Value::String(text) => Ok(Value::String(text.clone())),
        Value::Array(parts) => {
            let mut blocks = Vec::new();
            for part in parts {
                blocks.extend(translate_content_part(part)?);
            }
            Ok(Value::Array(blocks))
        }
        _ => Ok(Value::String(content.to_string())),
    }
}

fn translate_content_part(part: &Value) -> TranslationResult<Vec<Value>> {
    let Some(object) = part.as_object() else {
        return Ok(vec![text_block(part.to_string())]);
    };

    match object.get("type").and_then(Value::as_str) {
        Some("input_text" | "text" | "output_text") => Ok(vec![text_block(
            object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        )]),
        Some("refusal") => Ok(vec![text_block(
            object
                .get("refusal")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        )]),
        Some("input_image") => match object.get("image_url").and_then(Value::as_str) {
            Some(url) => Ok(vec![json!({
                "type": "image",
                "source": {
                    "type": "url",
                    "url": url
                }
            })]),
            None => Ok(vec![text_block(
                "[image omitted: only image_url is supported]".to_string(),
            )]),
        },
        Some("tool_use") => Ok(vec![part.clone()]),
        Some("tool_result") => Ok(vec![part.clone()]),
        Some(kind) => Ok(vec![text_block(format!(
            "[OpenAI Responses content `{kind}` omitted during Anthropic translation]"
        ))]),
        None => Ok(vec![text_block(part.to_string())]),
    }
}

fn translate_function_call(object: &Map<String, Value>) -> TranslationResult<Value> {
    let arguments = object
        .get("arguments")
        .and_then(Value::as_str)
        .unwrap_or("{}");
    let input = serde_json::from_str::<Value>(arguments)
        .unwrap_or_else(|_| Value::String(arguments.to_string()));
    Ok(json!({
        "type": "tool_use",
        "id": call_id(object),
        "name": object.get("name").and_then(Value::as_str).unwrap_or("function"),
        "input": input
    }))
}

fn translate_function_call_output(object: &Map<String, Value>) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": call_id(object),
        "content": translate_tool_output(object.get("output"))
    })
}

fn translate_custom_tool_call(object: &Map<String, Value>) -> Value {
    json!({
        "type": "tool_use",
        "id": call_id(object),
        "name": object.get("name").and_then(Value::as_str).unwrap_or("custom"),
        "input": object.get("input").cloned().unwrap_or_else(|| Value::String(String::new()))
    })
}

fn translate_custom_tool_call_output(object: &Map<String, Value>) -> Value {
    json!({
        "type": "tool_result",
        "tool_use_id": call_id(object),
        "content": translate_tool_output(object.get("output"))
    })
}

fn translate_tool_output(output: Option<&Value>) -> Value {
    match output {
        Some(Value::String(text)) => Value::String(text.clone()),
        Some(value) => Value::String(value.to_string()),
        None => Value::String(String::new()),
    }
}

fn translate_tools(tools: Option<&Value>) -> Option<Value> {
    let tools = tools?.as_array()?;
    let translated = tools.iter().filter_map(translate_tool).collect::<Vec<_>>();
    (!translated.is_empty()).then_some(Value::Array(translated))
}

fn translate_tool(tool: &Value) -> Option<Value> {
    let object = tool.as_object()?;
    match object.get("type").and_then(Value::as_str) {
        Some("function") => Some(json!({
            "type": "custom",
            "name": object.get("name").and_then(Value::as_str).unwrap_or("function"),
            "description": object.get("description").cloned().unwrap_or(Value::Null),
            "input_schema": anthropic_input_schema(object.get("parameters")),
        })),
        Some("custom") => Some(json!({
            "type": "custom",
            "name": object.get("name").and_then(Value::as_str).unwrap_or("custom"),
            "description": object.get("description").cloned().unwrap_or(Value::Null),
            "input_schema": json!({"type": "object", "properties": {}, "required": []}),
        })),
        _ => None,
    }
}

fn anthropic_input_schema(parameters: Option<&Value>) -> Value {
    let Some(Value::Object(parameters)) = parameters else {
        return json!({"type": "object", "properties": {}, "required": []});
    };

    let mut schema = parameters.clone();
    schema
        .entry("type".to_string())
        .or_insert_with(|| Value::String("object".to_string()));
    schema
        .entry("properties".to_string())
        .or_insert_with(|| json!({}));
    Value::Object(schema)
}

fn translate_tool_choice(
    choice: Option<&Value>,
    parallel_tool_calls: Option<bool>,
) -> Option<Value> {
    let disable_parallel_tool_use =
        (parallel_tool_calls == Some(false)).then_some(Value::Bool(true));
    let mut object = match choice? {
        Value::String(value) if value == "auto" => json!({"type": "auto"}),
        Value::String(value) if value == "none" => json!({"type": "none"}),
        Value::String(value) if value == "required" => json!({"type": "any"}),
        Value::Object(choice) => match choice.get("type").and_then(Value::as_str) {
            Some("function" | "custom") => json!({
                "type": "tool",
                "name": choice.get("name").and_then(Value::as_str).unwrap_or_default()
            }),
            Some("allowed_tools") => json!({"type": "any"}),
            Some("auto") => json!({"type": "auto"}),
            Some("none") => json!({"type": "none"}),
            Some("required") => json!({"type": "any"}),
            _ => return None,
        },
        _ => return None,
    };

    if let (Some(disable), Some(map)) = (disable_parallel_tool_use, object.as_object_mut())
        && map.get("type").and_then(Value::as_str) != Some("none")
    {
        map.insert("disable_parallel_tool_use".to_string(), disable);
    }
    Some(object)
}

fn translate_reasoning(reasoning: Option<&Value>) -> Option<Value> {
    let effort = reasoning?.get("effort").and_then(Value::as_str)?;
    match effort {
        "none" | "minimal" => Some(json!({"type": "disabled"})),
        "low" => Some(json!({"type": "enabled", "budget_tokens": 1024})),
        "medium" => Some(json!({"type": "enabled", "budget_tokens": 4096})),
        "high" => Some(json!({"type": "enabled", "budget_tokens": 8192})),
        "xhigh" => Some(json!({"type": "enabled", "budget_tokens": 16384})),
        _ => None,
    }
}

fn translate_metadata(metadata: Option<&Value>) -> Option<Value> {
    let metadata = metadata?.as_object()?;
    metadata
        .get("user_id")
        .and_then(Value::as_str)
        .map(|user_id| json!({"user_id": user_id}))
}

fn extract_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => join_text_parts(
            parts
                .iter()
                .filter_map(|part| {
                    part.as_object()
                        .and_then(|object| object.get("text"))
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect(),
        ),
        _ => None,
    }
}

fn join_text_parts(parts: Vec<String>) -> Option<String> {
    let text = parts
        .into_iter()
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

fn message(role: &str, content: Value) -> Value {
    json!({
        "role": role,
        "content": content,
    })
}

fn append_message_content_block(messages: &mut Vec<Value>, role: &str, block: Value) {
    // Used for translated tool turns. In Anthropic Messages, consecutive
    // tool_use blocks belong to one assistant turn and the matching
    // tool_result blocks belong to the immediately following user turn.
    // MiniMax M3 is stricter than some Anthropic-compatible upstreams about
    // this shape, so appending to the previous same-role message prevents
    // OpenAI Responses parallel tool items from becoming invalid standalone
    // assistant/user fragments.
    let Some(last) = messages.last_mut() else {
        messages.push(message(role, Value::Array(vec![block])));
        return;
    };
    if last.get("role").and_then(Value::as_str) != Some(role) {
        messages.push(message(role, Value::Array(vec![block])));
        return;
    }

    match last.get_mut("content") {
        Some(Value::Array(content)) => content.push(block),
        Some(Value::String(text)) => {
            let previous_text = std::mem::take(text);
            last["content"] = Value::Array(vec![text_block(previous_text), block]);
        }
        Some(content) => {
            let previous = std::mem::take(content);
            *content = Value::Array(vec![previous, block]);
        }
        None => {
            last["content"] = Value::Array(vec![block]);
        }
    }
}

fn text_block(text: String) -> Value {
    json!({
        "type": "text",
        "text": text,
    })
}

fn call_id(object: &Map<String, Value>) -> String {
    object
        .get("call_id")
        .or_else(|| object.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("tool_call")
        .to_string()
}

#[cfg(test)]
mod tests;
