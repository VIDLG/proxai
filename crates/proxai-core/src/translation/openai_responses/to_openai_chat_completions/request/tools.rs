use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::translation::TranslationScope;

pub(super) fn chat_tools(
    tools: Option<&[responses::Tool]>,
    scope: &TranslationScope,
) -> Option<Vec<chat::ChatCompletionTools>> {
    let translated = tools?
        .iter()
        .filter_map(|tool| chat_tool(tool, scope))
        .collect::<Vec<_>>();
    (!translated.is_empty()).then_some(translated)
}

fn chat_tool(
    tool: &responses::Tool,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionTools> {
    match tool {
        responses::Tool::Function(tool) => Some(chat::ChatCompletionTools::Function(
            chat_request::ChatCompletionTool {
                function: chat_request::FunctionObject {
                    name: tool.name.clone(),
                    parameters: tool.parameters.as_non_null().cloned(),
                    strict: tool.strict.as_non_null().copied().into(),
                    description: tool.description.as_non_null().cloned(),
                },
            },
        )),
        responses::Tool::Custom(tool) => Some(chat::ChatCompletionTools::Custom(
            chat_request::CustomToolChatCompletions {
                custom: chat_request::CustomToolProperties {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    format: tool.format.clone().map(Into::into),
                },
            },
        )),
        tool @ (responses::Tool::FileSearch(_)
        | responses::Tool::ComputerUsePreview(_)
        | responses::Tool::WebSearch(_)
        | responses::Tool::WebSearch20250826(_)
        | responses::Tool::Mcp(_)
        | responses::Tool::CodeInterpreter(_)
        | responses::Tool::ImageGeneration(_)
        | responses::Tool::LocalShell
        | responses::Tool::Shell(_)
        | responses::Tool::Computer(_)
        | responses::Tool::Namespace(_)
        | responses::Tool::ProgrammaticToolCalling
        | responses::Tool::ToolSearch(_)
        | responses::Tool::WebSearchPreview(_)
        | responses::Tool::WebSearchPreview20250311(_)
        | responses::Tool::ApplyPatch) => {
            scope.dropped(
                format!("Responses tool `{}`", tool.as_ref()),
                "Chat Completions has no matching tool type",
            );
            None
        }
    }
}

pub(super) fn chat_tool_choice(
    choice: Option<&responses::ToolChoiceParam>,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionToolChoiceOption> {
    let choice = choice?;
    match choice {
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::None) => Some(
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::None),
        ),
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Auto) => Some(
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Auto),
        ),
        responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Required) => Some(
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Required),
        ),
        responses::ToolChoiceParam::Function(choice) => {
            Some(chat::ChatCompletionToolChoiceOption::Function(
                chat_request::ChatCompletionNamedToolChoice {
                    function: chat_request::FunctionName {
                        name: choice.name.clone(),
                    },
                },
            ))
        }
        responses::ToolChoiceParam::Custom(choice) => {
            Some(chat::ChatCompletionToolChoiceOption::Custom(
                chat_request::ChatCompletionNamedToolChoiceCustom {
                    custom: chat_request::CustomName {
                        name: choice.name.clone(),
                    },
                },
            ))
        }
        responses::ToolChoiceParam::AllowedTools(allowed) => {
            Some(chat::ChatCompletionToolChoiceOption::AllowedTools(
                chat_request::ChatCompletionAllowedToolsChoice {
                    allowed_tools: chat_request::ChatCompletionAllowedTools {
                        mode: allowed.mode.into(),
                        tools: allowed.tools.clone(),
                    },
                },
            ))
        }
        responses::ToolChoiceParam::Hosted(choice) => {
            scope.dropped(
                format!("Responses tool_choice `{choice}`"),
                "Chat Completions has no matching tool choice",
            );
            None
        }
        choice @ (responses::ToolChoiceParam::Mcp(_)
        | responses::ToolChoiceParam::ApplyPatch
        | responses::ToolChoiceParam::Shell) => {
            scope.dropped(
                format!("Responses tool_choice `{}`", choice.as_ref()),
                "Chat Completions has no matching tool choice",
            );
            None
        }
    }
}
