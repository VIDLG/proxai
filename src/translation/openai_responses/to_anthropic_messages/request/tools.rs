use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::anthropic_messages::outbound::custom_tool;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&responses::Tool> for anthropic::ToolUnion {
    type Error = TranslationError;

    fn try_from(tool: &responses::Tool) -> TranslationResult<Self> {
        match tool {
            responses::Tool::Function(tool) => Ok(custom_tool(
                &tool.name,
                tool.description.clone(),
                tool.parameters.as_ref(),
                tool.strict,
                tool.defer_loading,
            )),
            responses::Tool::Custom(tool) => Ok(custom_tool(
                &tool.name,
                tool.description.clone(),
                None,
                None,
                tool.defer_loading,
            )),
            other => Err(TranslationError::InvalidPayload(format!(
                "OpenAI Responses tool `{}` cannot be translated to Anthropic Messages request tools",
                other.as_ref()
            ))),
        }
    }
}

pub(super) fn translate_tool_choice(
    choice: &responses::ToolChoiceParam,
    disable_parallel_tool_use: Option<bool>,
) -> TranslationResult<anthropic::ToolChoice> {
    match choice {
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Auto) => {
            Ok(anthropic::ToolChoice::Auto(anthropic::ToolChoiceAuto {
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::None) => {
            Ok(anthropic::ToolChoice::None(anthropic::ToolChoiceNone))
        }
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Required)
        | responses::ToolChoiceParam::AllowedTools(_) => {
            Ok(anthropic::ToolChoice::Any(anthropic::ToolChoiceAny {
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Function(choice) => {
            Ok(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.name.clone(),
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Custom(choice) => {
            Ok(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.name.clone(),
                disable_parallel_tool_use,
            }))
        }
        other => Err(TranslationError::InvalidPayload(format!(
            "OpenAI Responses tool_choice `{}` cannot be translated to Anthropic Messages request tool_choice",
            other.as_ref()
        ))),
    }
}
