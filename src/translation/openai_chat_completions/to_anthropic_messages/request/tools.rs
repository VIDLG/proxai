use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::anthropic_messages::outbound::custom_tool;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&chat::ChatCompletionTools> for anthropic::ToolUnion {
    type Error = TranslationError;

    fn try_from(tool: &chat::ChatCompletionTools) -> TranslationResult<Self> {
        match tool {
            chat::ChatCompletionTools::Function(tool) => Ok(custom_tool(
                &tool.function.name,
                tool.function.description.clone(),
                tool.function.parameters.as_ref(),
                tool.function.strict,
                None,
            )),
            chat::ChatCompletionTools::Custom(_) => Err(TranslationError::InvalidPayload(
                "Chat Completions custom tools cannot be translated to Anthropic Messages tools; Anthropic tools require JSON input_schema"
                    .to_string(),
            )),
        }
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
