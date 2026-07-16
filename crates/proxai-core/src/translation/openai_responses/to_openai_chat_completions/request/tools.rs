use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

impl TryFrom<&responses::Tool> for chat::ChatCompletionTools {
    type Error = TranslationError;

    fn try_from(value: &responses::Tool) -> TranslationResult<Self> {
        match value {
            responses::Tool::Function(tool) => {
                Ok(Self::Function(chat_request::ChatCompletionTool {
                    function: chat_request::FunctionObject {
                        name: tool.name.clone(),
                        parameters: tool.parameters.as_non_null().cloned(),
                        strict: tool.strict.as_non_null().copied().into(),
                        description: tool.description.as_non_null().cloned(),
                    },
                }))
            }
            responses::Tool::Custom(tool) => {
                Ok(Self::Custom(chat_request::CustomToolChatCompletions {
                    custom: chat_request::CustomToolProperties {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        format: tool.format.clone().map(Into::into),
                    },
                }))
            }
            // Hosted Responses tools (file_search, web_search, computer, mcp,
            // code_interpreter, image_generation, shell, apply_patch, etc.) have
            // no Chat Completions equivalent and are skipped at the call site.
            other => Err(TranslationError::InvalidPayload(format!(
                "OpenAI Responses tool `{}` cannot be translated to Chat Completions",
                other.as_ref()
            ))),
        }
    }
}

impl From<responses::CustomToolParamFormat> for chat_request::CustomToolPropertiesFormat {
    fn from(value: responses::CustomToolParamFormat) -> Self {
        match value {
            responses::CustomToolParamFormat::Text => Self::Text,
            responses::CustomToolParamFormat::Grammar(grammar) => {
                Self::Grammar(chat_request::CustomGrammarFormatParam {
                    definition: grammar.definition,
                    syntax: grammar.syntax.into(),
                })
            }
        }
    }
}

impl From<responses::GrammarSyntax> for chat_request::GrammarSyntax {
    fn from(value: responses::GrammarSyntax) -> Self {
        match value {
            responses::GrammarSyntax::Lark => Self::Lark,
            responses::GrammarSyntax::Regex => Self::Regex,
        }
    }
}

pub(super) fn chat_tools(
    tools: Option<&[responses::Tool]>,
    scope: &TranslationScope,
) -> Option<Vec<chat::ChatCompletionTools>> {
    let tools = tools?;

    let mut translated = Vec::new();
    for tool in tools {
        match chat::ChatCompletionTools::try_from(tool) {
            Ok(tool) => translated.push(tool),
            Err(error) => scope.dropped(
                format!("Responses tool `{}`", tool.as_ref()),
                error.to_string(),
            ),
        }
    }
    (!translated.is_empty()).then_some(translated)
}

impl TryFrom<&responses::ToolChoiceParam> for chat::ChatCompletionToolChoiceOption {
    type Error = TranslationError;

    fn try_from(value: &responses::ToolChoiceParam) -> TranslationResult<Self> {
        match value {
            responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::None) => {
                Ok(Self::Mode(chat::ToolChoiceOptions::None))
            }
            responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Auto) => {
                Ok(Self::Mode(chat::ToolChoiceOptions::Auto))
            }
            responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Required) => {
                Ok(Self::Mode(chat::ToolChoiceOptions::Required))
            }
            responses::ToolChoiceParam::Function(choice) => Ok(Self::Function(
                chat_request::ChatCompletionNamedToolChoice {
                    function: chat_request::FunctionName {
                        name: choice.name.clone(),
                    },
                },
            )),
            responses::ToolChoiceParam::Custom(choice) => Ok(Self::Custom(
                chat_request::ChatCompletionNamedToolChoiceCustom {
                    custom: chat_request::CustomName {
                        name: choice.name.clone(),
                    },
                },
            )),
            responses::ToolChoiceParam::AllowedTools(allowed) => Ok(Self::AllowedTools(
                chat_request::ChatCompletionAllowedToolsChoice {
                    allowed_tools: chat_request::ChatCompletionAllowedTools {
                        mode: allowed.mode.into(),
                        tools: allowed.tools.clone(),
                    },
                },
            )),
            // Hosted tool choices (apply_patch, shell, mcp, file_search, etc.)
            // have no Chat representation.
            other => Err(TranslationError::InvalidPayload(format!(
                "OpenAI Responses tool_choice `{}` cannot be translated to Chat Completions",
                tool_choice_name(other)
            ))),
        }
    }
}

impl From<responses::ToolChoiceAllowedMode> for chat_request::ToolChoiceAllowedMode {
    fn from(value: responses::ToolChoiceAllowedMode) -> Self {
        match value {
            responses::ToolChoiceAllowedMode::Auto => Self::Auto,
            responses::ToolChoiceAllowedMode::Required => Self::Required,
        }
    }
}

pub(super) fn chat_tool_choice(
    choice: Option<&responses::ToolChoiceParam>,
    scope: &TranslationScope,
) -> Option<chat::ChatCompletionToolChoiceOption> {
    let choice = choice?;
    match chat::ChatCompletionToolChoiceOption::try_from(choice) {
        Ok(choice) => Some(choice),
        Err(error) => {
            scope.dropped(
                format!("Responses tool_choice `{}`", tool_choice_name(choice)),
                error.to_string(),
            );
            None
        }
    }
}

fn tool_choice_name(choice: &responses::ToolChoiceParam) -> String {
    match choice {
        responses::ToolChoiceParam::Hosted(choice) => choice.to_string(),
        responses::ToolChoiceParam::Mode(choice) => choice.to_string(),
        other => other.as_ref().to_string(),
    }
}
