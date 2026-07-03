use super::types::tool_discriminant;
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::anthropic_messages::outbound::custom_tool;

pub(super) fn translate_tools(
    tools: Option<&Vec<responses::Tool>>,
) -> Option<Vec<anthropic::ToolUnion>> {
    let translated = tools?.iter().filter_map(translate_tool).collect::<Vec<_>>();
    (!translated.is_empty()).then_some(translated)
}

fn translate_tool(tool: &responses::Tool) -> Option<anthropic::ToolUnion> {
    match tool {
        responses::Tool::Function(tool) => Some(custom_tool(
            &tool.name,
            tool.description.clone(),
            tool.parameters.as_ref(),
            tool.strict,
            tool.defer_loading,
        )),
        responses::Tool::Custom(tool) => Some(custom_tool(
            &tool.name,
            tool.description.clone(),
            None,
            None,
            tool.defer_loading,
        )),
        other => {
            tracing::trace!(
                tool_type = tool_discriminant(other),
                reason = "Responses tool has no Anthropic Messages request representation"
            );
            None
        }
    }
}

pub(super) fn translate_tool_choice(
    choice: Option<&responses::ToolChoiceParam>,
    parallel_tool_calls: Option<bool>,
) -> Option<anthropic::ToolChoice> {
    let disable_parallel_tool_use = (parallel_tool_calls == Some(false)).then_some(true);

    match choice? {
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Auto) => {
            Some(anthropic::ToolChoice::Auto(anthropic::ToolChoiceAuto {
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::None) => {
            Some(anthropic::ToolChoice::None(anthropic::ToolChoiceNone))
        }
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Required)
        | responses::ToolChoiceParam::AllowedTools(_) => {
            Some(anthropic::ToolChoice::Any(anthropic::ToolChoiceAny {
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Function(choice) => {
            Some(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.name.clone(),
                disable_parallel_tool_use,
            }))
        }
        responses::ToolChoiceParam::Custom(choice) => {
            Some(anthropic::ToolChoice::Tool(anthropic::ToolChoiceTool {
                name: choice.name.clone(),
                disable_parallel_tool_use,
            }))
        }
        other => {
            tracing::trace!(
                tool_choice_type = tool_choice_discriminant(other),
                reason = "Responses tool_choice has no Anthropic Messages request representation"
            );
            None
        }
    }
}

fn tool_choice_discriminant(choice: &responses::ToolChoiceParam) -> &'static str {
    match choice {
        responses::ToolChoiceParam::AllowedTools(_) => "allowed_tools",
        responses::ToolChoiceParam::Function(_) => "function",
        responses::ToolChoiceParam::Mcp(_) => "mcp",
        responses::ToolChoiceParam::Custom(_) => "custom",
        responses::ToolChoiceParam::ApplyPatch => "apply_patch",
        responses::ToolChoiceParam::Shell => "shell",
        responses::ToolChoiceParam::Hosted(_) => "hosted",
        responses::ToolChoiceParam::Mode(_) => "mode",
    }
}
