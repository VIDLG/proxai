//! `openai_chat_completions -> anthropic_messages` request translation.

use serde_json::{Map, Number, Value, json};

use crate::error::{InternalError, Result};
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;

const DEFAULT_MAX_TOKENS: u32 = 4096;

pub(crate) fn translate_request_payload(
    payload: &Value,
    request_model: &str,
    upstream_model: &str,
) -> Result<Value, InternalError> {
    let request = serde_json::from_value::<chat::CreateChatCompletionRequest>(payload.clone())?;
    let translated = translate_request(&request, request_model, upstream_model);
    Ok(serde_json::to_value(translated)?)
}

fn translate_request(
    request: &chat::CreateChatCompletionRequest,
    request_model: &str,
    upstream_model: &str,
) -> anthropic::MessageCreateParamsBase {
    let mut system_parts = Vec::new();
    let mut messages = Vec::new();

    for message in &request.messages {
        match message {
            chat::ChatCompletionRequestMessage::Developer(message) => {
                collect_developer_content(&message.content, &mut system_parts);
            }
            chat::ChatCompletionRequestMessage::System(message) => {
                collect_system_content(&message.content, &mut system_parts);
            }
            chat::ChatCompletionRequestMessage::User(message) => {
                messages.push(anthropic_message(
                    anthropic::Role::User,
                    user_content(&message.content),
                ));
            }
            chat::ChatCompletionRequestMessage::Assistant(message) => {
                messages.push(anthropic_message(
                    anthropic::Role::Assistant,
                    assistant_content(message),
                ));
            }
            chat::ChatCompletionRequestMessage::Tool(message) => {
                messages.push(anthropic_message(
                    anthropic::Role::User,
                    anthropic::MessageParamContent::Blocks(vec![
                        anthropic::ContentBlockParam::ToolResult(anthropic::ToolResultBlockParam {
                            tool_use_id: message.tool_call_id.clone(),
                            content: Some(Value::String(tool_content_as_text(&message.content))),
                            is_error: Some(false),
                            cache_control: None,
                        }),
                    ]),
                ));
            }
            chat::ChatCompletionRequestMessage::Function(message) => {
                messages.push(anthropic_message(
                    anthropic::Role::User,
                    anthropic::MessageParamContent::Text(
                        message.content.clone().unwrap_or_default(),
                    ),
                ));
            }
        }
    }

    if messages.is_empty() {
        messages.push(anthropic_message(
            anthropic::Role::User,
            anthropic::MessageParamContent::Text(String::new()),
        ));
    }

    anthropic::MessageCreateParamsBase {
        max_tokens: request
            .max_completion_tokens
            .or(request.max_tokens)
            .unwrap_or(DEFAULT_MAX_TOKENS),
        messages,
        model: if upstream_model != request_model {
            upstream_model.to_string()
        } else {
            request_model.to_string()
        },
        cache_control: None,
        container: None,
        inference_geo: None,
        metadata: None,
        output_config: None,
        service_tier: None,
        stop_sequences: stop_sequences(request.stop.as_ref()),
        stream: request.stream,
        system: join_text_parts(system_parts).map(anthropic::SystemPrompt::Text),
        temperature: request.temperature.and_then(number_from_f32),
        thinking: None,
        tool_choice: request.tool_choice.as_ref().and_then(translate_tool_choice),
        tools: request
            .tools
            .as_ref()
            .and_then(|tools| translate_tools(tools)),
        top_k: None,
        top_p: request.top_p.and_then(number_from_f32),
    }
}

fn anthropic_message(
    role: anthropic::Role,
    content: anthropic::MessageParamContent,
) -> anthropic::MessageParam {
    anthropic::MessageParam { role, content }
}

fn number_from_f32(value: f32) -> Option<Number> {
    serde_json::Number::from_f64(value as f64)
}

fn stop_sequences(value: Option<&chat::StopConfiguration>) -> Option<Vec<String>> {
    match value? {
        chat::StopConfiguration::String(value) if !value.is_empty() => Some(vec![value.clone()]),
        chat::StopConfiguration::StringArray(values) => {
            let values = values
                .iter()
                .filter(|value| !value.is_empty())
                .cloned()
                .collect::<Vec<_>>();
            (!values.is_empty()).then_some(values)
        }
        _ => None,
    }
}

fn join_text_parts(parts: Vec<String>) -> Option<String> {
    let text = parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

fn collect_developer_content(
    content: &chat::ChatCompletionRequestDeveloperMessageContent,
    out: &mut Vec<String>,
) {
    match content {
        chat::ChatCompletionRequestDeveloperMessageContent::Text(text) => out.push(text.clone()),
        chat::ChatCompletionRequestDeveloperMessageContent::Array(parts) => {
            out.extend(parts.iter().map(|part| match part {
                chat::ChatCompletionRequestDeveloperMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            }));
        }
    }
}

fn collect_system_content(
    content: &chat::ChatCompletionRequestSystemMessageContent,
    out: &mut Vec<String>,
) {
    match content {
        chat::ChatCompletionRequestSystemMessageContent::Text(text) => out.push(text.clone()),
        chat::ChatCompletionRequestSystemMessageContent::Array(parts) => {
            out.extend(parts.iter().map(|part| match part {
                chat::ChatCompletionRequestSystemMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            }));
        }
    }
}

fn text_block(text: String) -> anthropic::ContentBlockParam {
    anthropic::ContentBlockParam::Text(anthropic::TextBlockParam {
        text,
        cache_control: None,
        citations: None,
    })
}

fn user_content(
    content: &chat::ChatCompletionRequestUserMessageContent,
) -> anthropic::MessageParamContent {
    match content {
        chat::ChatCompletionRequestUserMessageContent::Text(text) => {
            anthropic::MessageParamContent::Text(text.clone())
        }
        chat::ChatCompletionRequestUserMessageContent::Array(parts) => {
            let blocks = parts
                .iter()
                .filter_map(|part| match part {
                    chat::ChatCompletionRequestUserMessageContentPart::Text(part) => {
                        Some(text_block(part.text.clone()))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            if blocks.is_empty() {
                anthropic::MessageParamContent::Text(String::new())
            } else {
                anthropic::MessageParamContent::Blocks(blocks)
            }
        }
    }
}

fn assistant_content(
    message: &chat::ChatCompletionRequestAssistantMessage,
) -> anthropic::MessageParamContent {
    let mut blocks = Vec::new();
    if let Some(content) = &message.content {
        match content {
            chat::ChatCompletionRequestAssistantMessageContent::Text(text) => {
                if !text.is_empty() {
                    blocks.push(text_block(text.clone()));
                }
            }
            chat::ChatCompletionRequestAssistantMessageContent::Array(parts) => {
                for part in parts {
                    if let chat::ChatCompletionRequestAssistantMessageContentPart::Text(part) = part
                    {
                        blocks.push(text_block(part.text.clone()));
                    }
                }
            }
        }
    }

    for tool_call in message.tool_calls.iter().flatten() {
        match tool_call {
            chat::ChatCompletionMessageToolCalls::Function(tool_call) => {
                let input = serde_json::from_str::<Value>(&tool_call.function.arguments)
                    .unwrap_or_else(|_| json!({}));
                blocks.push(anthropic::ContentBlockParam::ToolUse(
                    anthropic::ToolUseBlockParam {
                        id: tool_call.id.clone(),
                        input,
                        name: tool_call.function.name.clone(),
                        cache_control: None,
                        caller: None,
                    },
                ));
            }
            chat::ChatCompletionMessageToolCalls::Custom(tool_call) => {
                blocks.push(anthropic::ContentBlockParam::ToolUse(
                    anthropic::ToolUseBlockParam {
                        id: tool_call.id.clone(),
                        input: Value::String(tool_call.custom_tool.input.clone()),
                        name: tool_call.custom_tool.name.clone(),
                        cache_control: None,
                        caller: None,
                    },
                ));
            }
        }
    }

    if blocks.is_empty() {
        anthropic::MessageParamContent::Text(String::new())
    } else {
        anthropic::MessageParamContent::Blocks(blocks)
    }
}

fn tool_content_as_text(content: &chat::ChatCompletionRequestToolMessageContent) -> String {
    match content {
        chat::ChatCompletionRequestToolMessageContent::Text(text) => text.clone(),
        chat::ChatCompletionRequestToolMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestToolMessageContentPart::Text(part) => part.text.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}

fn translate_tools(tools: &[chat::ChatCompletionTools]) -> Option<Vec<anthropic::ToolUnion>> {
    let tools = tools
        .iter()
        .map(|tool| match tool {
            chat::ChatCompletionTools::Function(tool) => {
                anthropic::ToolUnion::Custom(anthropic::Tool {
                    input_schema: anthropic_input_schema(tool.function.parameters.as_ref()),
                    name: tool.function.name.clone(),
                    allowed_callers: None,
                    cache_control: None,
                    defer_loading: None,
                    description: tool.function.description.clone(),
                    eager_input_streaming: None,
                    input_examples: None,
                    strict: tool.function.strict,
                    type_: None,
                })
            }
            chat::ChatCompletionTools::Custom(tool) => {
                anthropic::ToolUnion::Custom(anthropic::Tool {
                    input_schema: anthropic::InputSchema {
                        type_: "object".to_string(),
                        properties: Some(json!({})),
                        required: Some(Vec::new()),
                        extra: json!({}),
                    },
                    name: tool.custom.name.clone(),
                    allowed_callers: None,
                    cache_control: None,
                    defer_loading: None,
                    description: tool.custom.description.clone(),
                    eager_input_streaming: None,
                    input_examples: None,
                    strict: None,
                    type_: None,
                })
            }
        })
        .collect::<Vec<_>>();
    (!tools.is_empty()).then_some(tools)
}

fn anthropic_input_schema(parameters: Option<&Value>) -> anthropic::InputSchema {
    let Some(Value::Object(parameters)) = parameters else {
        return anthropic::InputSchema {
            type_: "object".to_string(),
            properties: Some(json!({})),
            required: Some(Vec::new()),
            extra: json!({}),
        };
    };

    let mut extra = MapWithoutSchemaCore::from(parameters.clone());
    let type_ = extra
        .remove_string("type")
        .unwrap_or_else(|| "object".to_string());
    let properties = extra.remove("properties").or_else(|| Some(json!({})));
    let required = extra
        .remove("required")
        .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
        .or_else(|| Some(Vec::new()));

    anthropic::InputSchema {
        type_,
        properties,
        required,
        extra: Value::Object(extra.into_inner()),
    }
}

struct MapWithoutSchemaCore(Map<String, Value>);

impl MapWithoutSchemaCore {
    fn from(map: Map<String, Value>) -> Self {
        Self(map)
    }

    fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.remove(key)
    }

    fn remove_string(&mut self, key: &str) -> Option<String> {
        self.remove(key).and_then(|value| match value {
            Value::String(value) => Some(value),
            _ => None,
        })
    }

    fn into_inner(self) -> Map<String, Value> {
        self.0
    }
}

fn translate_tool_choice(
    value: &chat::ChatCompletionToolChoiceOption,
) -> Option<anthropic::ToolChoice> {
    match value {
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Auto) => {
            Some(anthropic::ToolChoice::Auto(anthropic::ToolChoiceAuto {
                disable_parallel_tool_use: None,
            }))
        }
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Required) => {
            Some(anthropic::ToolChoice::Any(anthropic::ToolChoiceAny {
                disable_parallel_tool_use: None,
            }))
        }
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::None) => None,
        chat::ChatCompletionToolChoiceOption::Function(choice) => {
            Some(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.function.name.clone(),
                disable_parallel_tool_use: None,
            }))
        }
        chat::ChatCompletionToolChoiceOption::Custom(choice) => {
            Some(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.custom.name.clone(),
                disable_parallel_tool_use: None,
            }))
        }
        chat::ChatCompletionToolChoiceOption::AllowedTools(_) => {
            Some(anthropic::ToolChoice::Any(anthropic::ToolChoiceAny {
                disable_parallel_tool_use: None,
            }))
        }
    }
}

#[cfg(test)]
#[path = "to_anthropic_messages_tests.rs"]
mod tests;
