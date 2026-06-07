//! Request-level conversion for `openai_chat_completions -> anthropic_messages`.

use serde_json::{Map, Number, Value, json};

use super::content::{
    anthropic_message, assistant_content, collect_developer_content, collect_system_content,
    join_text_parts, tool_content_as_text, user_content,
};
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::TranslationResult;

const DEFAULT_MAX_TOKENS: u32 = 4096;

impl TryFrom<&chat::CreateChatCompletionRequest> for anthropic::MessageCreateParamsBase {
    type Error = crate::translation::TranslationError;

    fn try_from(request: &chat::CreateChatCompletionRequest) -> TranslationResult<Self> {
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
                        user_content(&message.content)?,
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
                            anthropic::ContentBlockParam::ToolResult(
                                anthropic::ToolResultBlockParam {
                                    tool_use_id: message.tool_call_id.clone(),
                                    content: Some(anthropic::ToolResultContentParam::Text(
                                        tool_content_as_text(&message.content),
                                    )),
                                    is_error: Some(false),
                                    cache_control: None,
                                },
                            ),
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

        Ok(Self {
            max_tokens: request
                .max_completion_tokens
                .or(request.max_tokens)
                .unwrap_or(DEFAULT_MAX_TOKENS),
            messages,
            model: request.model.clone(),
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
            tools: request.tools.as_ref().and_then(|tools| {
                let tools = tools.iter().map(Into::into).collect::<Vec<_>>();
                (!tools.is_empty()).then_some(tools)
            }),
            top_k: None,
            top_p: request.top_p.and_then(number_from_f32),
        })
    }
}

impl From<&chat::ChatCompletionTools> for anthropic::ToolUnion {
    fn from(tool: &chat::ChatCompletionTools) -> Self {
        match tool {
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
                    input_schema: empty_input_schema(),
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
        }
    }
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

fn empty_input_schema() -> anthropic::InputSchema {
    anthropic::InputSchema {
        type_: "object".to_string(),
        properties: Some(json!({})),
        required: Some(Vec::new()),
        extra: json!({}),
    }
}

fn anthropic_input_schema(parameters: Option<&Value>) -> anthropic::InputSchema {
    let Some(Value::Object(parameters)) = parameters else {
        return empty_input_schema();
    };

    let mut extra = parameters.clone();
    let type_ = remove_string(&mut extra, "type").unwrap_or_else(|| "object".to_string());
    let properties = extra.remove("properties").or_else(|| Some(json!({})));
    let required = extra
        .remove("required")
        .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
        .or_else(|| Some(Vec::new()));

    anthropic::InputSchema {
        type_,
        properties,
        required,
        extra: Value::Object(extra),
    }
}

fn remove_string(map: &mut Map<String, Value>, key: &str) -> Option<String> {
    map.remove(key).and_then(|value| match value {
        Value::String(value) => Some(value),
        _ => None,
    })
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
