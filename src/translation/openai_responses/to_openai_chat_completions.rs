//! `openai_responses -> openai_chat_completions` request translation.
//!
//! The Responses `input` field is intentionally handled as JSON because it can
//! contain future or provider-specific items. The generated Chat Completions
//! payload is validated by the provider request preparation path.

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::{InternalError, Result};
use crate::protocol::openai::responses::{Reasoning, ReasoningEffort};

#[derive(Debug, Default, Deserialize)]
struct ResponsesRequestView {
    input: Option<Value>,
    instructions: Option<String>,
    max_output_tokens: Option<u32>,
    parallel_tool_calls: Option<bool>,
    reasoning: Option<Reasoning>,
    stream: Option<bool>,
    temperature: Option<f32>,
    tool_choice: Option<Value>,
    tools: Option<Value>,
    top_p: Option<f32>,
}

pub(crate) fn translate_request_payload(
    payload: &Value,
    request_model: &str,
    upstream_model: &str,
) -> Result<Value, InternalError> {
    let source = serde_json::from_value::<ResponsesRequestView>(payload.clone())?;

    let mut messages = Vec::new();
    if let Some(instructions) = source.instructions.as_deref() {
        if !instructions.trim().is_empty() {
            messages.push(json!({
                "role": "system",
                "content": instructions,
            }));
        }
    }
    messages.extend(translate_input(source.input.as_ref())?);
    if messages.is_empty() {
        messages.push(json!({
            "role": "user",
            "content": "",
        }));
    }

    let mut request = Map::new();
    request.insert(
        "model".to_string(),
        Value::String(if upstream_model != request_model {
            upstream_model.to_string()
        } else {
            request_model.to_string()
        }),
    );
    request.insert("messages".to_string(), Value::Array(messages));

    if let Some(max_tokens) = source.max_output_tokens {
        request.insert("max_completion_tokens".to_string(), json!(max_tokens));
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
    if let Some(parallel_tool_calls) = source.parallel_tool_calls {
        request.insert(
            "parallel_tool_calls".to_string(),
            Value::Bool(parallel_tool_calls),
        );
    }
    if let Some(reasoning_effort) = source.reasoning.and_then(|reasoning| reasoning.effort) {
        request.insert(
            "reasoning_effort".to_string(),
            Value::String(chat_reasoning_effort(reasoning_effort).to_string()),
        );
    }
    if let Some(tools) = translate_tools(source.tools.as_ref()) {
        request.insert("tools".to_string(), tools);
    }
    if let Some(tool_choice) = translate_tool_choice(source.tool_choice.as_ref()) {
        request.insert("tool_choice".to_string(), tool_choice);
    }

    Ok(Value::Object(request))
}

fn translate_input(input: Option<&Value>) -> Result<Vec<Value>, InternalError> {
    let Some(input) = input else {
        return Ok(Vec::new());
    };

    match input {
        Value::String(text) => Ok(vec![json!({"role": "user", "content": text})]),
        Value::Array(items) => {
            let mut messages = Vec::new();
            for item in items {
                translate_input_item(item, &mut messages)?;
            }
            Ok(messages)
        }
        other => Ok(vec![json!({"role": "user", "content": other.to_string()})]),
    }
}

fn translate_input_item(item: &Value, messages: &mut Vec<Value>) -> Result<(), InternalError> {
    let Some(object) = item.as_object() else {
        messages.push(json!({"role": "user", "content": item.to_string()}));
        return Ok(());
    };

    match object.get("type").and_then(Value::as_str) {
        Some("message") | None => translate_message_object(object, messages),
        Some("function_call") => {
            append_assistant_tool_call(messages, translate_function_call(object));
            Ok(())
        }
        Some("function_call_output") => {
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id(object),
                "content": stringify_tool_output(object.get("output")),
            }));
            Ok(())
        }
        Some("custom_tool_call") => {
            append_assistant_tool_call(messages, translate_custom_tool_call(object));
            Ok(())
        }
        Some("custom_tool_call_output") => {
            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id(object),
                "content": stringify_tool_output(object.get("output")),
            }));
            Ok(())
        }
        Some(kind) => {
            messages.push(json!({
                "role": "user",
                "content": format!(
                    "[OpenAI Responses item `{kind}` omitted during Chat Completions translation]"
                ),
            }));
            Ok(())
        }
    }
}

fn translate_message_object(
    object: &Map<String, Value>,
    messages: &mut Vec<Value>,
) -> Result<(), InternalError> {
    let role = object.get("role").and_then(Value::as_str).unwrap_or("user");
    let chat_role = match role {
        "system" => "system",
        "developer" => "developer",
        "assistant" => "assistant",
        "tool" => "tool",
        _ => "user",
    };
    let empty_content = Value::String(String::new());
    let content = object.get("content").unwrap_or(&empty_content);
    let mut message = Map::new();
    message.insert("role".to_string(), Value::String(chat_role.to_string()));
    message.insert("content".to_string(), translate_message_content(content));
    if chat_role == "tool" {
        if let Some(tool_call_id) = object.get("tool_call_id").and_then(Value::as_str) {
            message.insert(
                "tool_call_id".to_string(),
                Value::String(tool_call_id.to_string()),
            );
        }
    }
    messages.push(Value::Object(message));
    Ok(())
}

fn translate_message_content(content: &Value) -> Value {
    match content {
        Value::String(text) => Value::String(text.clone()),
        Value::Array(parts) => {
            let mut translated = Vec::new();
            for part in parts {
                translated.extend(translate_content_part(part));
            }
            if translated.is_empty() {
                Value::String(String::new())
            } else {
                Value::Array(translated)
            }
        }
        other => Value::String(other.to_string()),
    }
}

fn translate_content_part(part: &Value) -> Vec<Value> {
    let Some(object) = part.as_object() else {
        return vec![json!({"type": "text", "text": part.to_string()})];
    };

    match object.get("type").and_then(Value::as_str) {
        Some("input_text" | "text" | "output_text") => vec![json!({
            "type": "text",
            "text": object.get("text").and_then(Value::as_str).unwrap_or_default(),
        })],
        Some("refusal") => vec![json!({
            "type": "text",
            "text": object
                .get("refusal")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        })],
        Some("input_image") => {
            let Some(url) = object.get("image_url").and_then(Value::as_str) else {
                return vec![json!({
                    "type": "text",
                    "text": "[image omitted: only image_url is supported]",
                })];
            };
            let mut image_url = Map::new();
            image_url.insert("url".to_string(), Value::String(url.to_string()));
            if let Some(detail) = object.get("detail").and_then(Value::as_str) {
                image_url.insert("detail".to_string(), Value::String(detail.to_string()));
            }
            vec![json!({"type": "image_url", "image_url": image_url})]
        }
        Some("input_file") => vec![json!({
            "type": "text",
            "text": "[file omitted during Chat Completions translation]",
        })],
        Some(kind) => vec![json!({
            "type": "text",
            "text": format!(
                "[OpenAI Responses content `{kind}` omitted during Chat Completions translation]"
            ),
        })],
        None => vec![json!({"type": "text", "text": part.to_string()})],
    }
}

fn append_assistant_tool_call(messages: &mut Vec<Value>, tool_call: Value) {
    if let Some(last) = messages.last_mut().and_then(Value::as_object_mut) {
        if last.get("role").and_then(Value::as_str) == Some("assistant") {
            let tool_calls = last
                .entry("tool_calls".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Some(tool_calls) = tool_calls.as_array_mut() {
                tool_calls.push(tool_call);
                return;
            }
        }
    }

    messages.push(json!({
        "role": "assistant",
        "content": null,
        "tool_calls": [tool_call],
    }));
}

fn translate_function_call(object: &Map<String, Value>) -> Value {
    json!({
        "id": call_id(object),
        "type": "function",
        "function": {
            "name": object.get("name").and_then(Value::as_str).unwrap_or("function"),
            "arguments": object
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}"),
        }
    })
}

fn translate_custom_tool_call(object: &Map<String, Value>) -> Value {
    json!({
        "id": call_id(object),
        "type": "custom",
        "custom_tool": {
            "name": object.get("name").and_then(Value::as_str).unwrap_or("custom"),
            "input": stringify_tool_output(object.get("input")),
        }
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

fn stringify_tool_output(output: Option<&Value>) -> String {
    match output {
        Some(Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

fn translate_tools(tools: Option<&Value>) -> Option<Value> {
    let tools = tools?.as_array()?;
    let translated = tools.iter().filter_map(translate_tool).collect::<Vec<_>>();
    (!translated.is_empty()).then_some(Value::Array(translated))
}

fn translate_tool(tool: &Value) -> Option<Value> {
    let object = tool.as_object()?;
    match object.get("type").and_then(Value::as_str)? {
        "function" => Some(json!({
            "type": "function",
            "function": {
                "name": object.get("name")?.clone(),
                "description": object.get("description").cloned(),
                "parameters": object
                    .get("parameters")
                    .or_else(|| object.get("input_schema"))
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object", "properties": {}})),
                "strict": object.get("strict").cloned(),
            }
        })),
        "custom" => Some(json!({
            "type": "custom",
            "custom": {
                "name": object.get("name")?.clone(),
                "description": object.get("description").cloned(),
                "format": object
                    .get("format")
                    .cloned()
                    .unwrap_or_else(|| Value::String("text".to_string())),
            }
        })),
        _ => None,
    }
}

fn translate_tool_choice(choice: Option<&Value>) -> Option<Value> {
    let choice = choice?;
    match choice {
        Value::String(value) if matches!(value.as_str(), "auto" | "none") => Some(choice.clone()),
        Value::String(value) if value == "required" => Some(Value::String("required".to_string())),
        Value::Object(object) => match object.get("type").and_then(Value::as_str) {
            Some("function") => Some(json!({
                "type": "function",
                "function": {
                    "name": object.get("name")?.clone(),
                }
            })),
            Some("custom") => Some(json!({
                "type": "custom",
                "custom": {
                    "name": object.get("name")?.clone(),
                }
            })),
            Some("allowed_tools") => Some(Value::String("required".to_string())),
            Some("auto") => Some(Value::String("auto".to_string())),
            Some("none") => Some(Value::String("none".to_string())),
            Some("required") => Some(Value::String("required".to_string())),
            _ => None,
        },
        _ => None,
    }
}

fn chat_reasoning_effort(effort: ReasoningEffort) -> &'static str {
    match effort {
        ReasoningEffort::None => "none",
        ReasoningEffort::Minimal => "minimal",
        ReasoningEffort::Low => "low",
        ReasoningEffort::Medium => "medium",
        ReasoningEffort::High => "high",
        ReasoningEffort::Xhigh => "xhigh",
    }
}

#[cfg(test)]
#[path = "to_openai_chat_completions_tests.rs"]
mod tests;
