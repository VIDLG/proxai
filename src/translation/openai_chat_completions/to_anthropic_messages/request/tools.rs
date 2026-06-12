use serde_json::{Value, json};

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;

use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&chat::ChatCompletionTools> for anthropic::ToolUnion {
    type Error = TranslationError;

    fn try_from(tool: &chat::ChatCompletionTools) -> TranslationResult<Self> {
        match tool {
            chat::ChatCompletionTools::Function(tool) => {
                Ok(anthropic::ToolUnion::Custom(anthropic::Tool {
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
                }))
            }
            chat::ChatCompletionTools::Custom(_) => Err(TranslationError::InvalidPayload(
                "Chat Completions custom tools cannot be translated to Anthropic Messages tools; Anthropic tools require JSON input_schema"
                    .to_string(),
            )),
        }
    }
}

fn anthropic_input_schema(parameters: Option<&Value>) -> anthropic::InputSchema {
    let Some(Value::Object(parameters)) = parameters else {
        return anthropic::InputSchema::default();
    };

    let mut extra = parameters.clone();
    let type_ = match extra.remove("type") {
        Some(Value::String(value)) => value,
        _ => "object".to_string(),
    };
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

pub(super) fn translate_tool_choice(
    value: &chat::ChatCompletionToolChoiceOption,
) -> TranslationResult<Option<anthropic::ToolChoice>> {
    match value {
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Auto) => {
            Ok(Some(anthropic::ToolChoice::Auto(anthropic::ToolChoiceAuto {
                disable_parallel_tool_use: None,
            })))
        }
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Required) => {
            Ok(Some(anthropic::ToolChoice::Any(anthropic::ToolChoiceAny {
                disable_parallel_tool_use: None,
            })))
        }
        chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::None) => {
            Ok(Some(anthropic::ToolChoice::None(anthropic::ToolChoiceNone)))
        }
        chat::ChatCompletionToolChoiceOption::Function(choice) => {
            Ok(Some(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.function.name.clone(),
                disable_parallel_tool_use: None,
            })))
        }
        chat::ChatCompletionToolChoiceOption::Custom(_) => Err(TranslationError::InvalidPayload(
            "Chat Completions custom tool choices cannot be translated to Anthropic Messages tool_choice"
                .to_string(),
        )),
        chat::ChatCompletionToolChoiceOption::AllowedTools(_) => Err(
            TranslationError::InvalidPayload(
                "Chat Completions allowed_tools tool choices cannot be translated to Anthropic Messages tool_choice"
                    .to_string(),
            ),
        ),
    }
}
