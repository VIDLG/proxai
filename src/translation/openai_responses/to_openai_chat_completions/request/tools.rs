use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&responses::Tool> for chat::ChatCompletionTools {
    type Error = TranslationError;

    fn try_from(value: &responses::Tool) -> TranslationResult<Self> {
        match value {
            responses::Tool::Function(tool) => {
                Ok(Self::Function(chat_request::ChatCompletionTool {
                    function: chat_request::FunctionObject {
                        name: tool.name.clone(),
                        parameters: tool.parameters.clone(),
                        strict: tool.strict,
                        description: tool.description.clone(),
                    },
                }))
            }
            responses::Tool::Custom(tool) => {
                Ok(Self::Custom(chat_request::CustomToolChatCompletions {
                    custom: chat_request::CustomToolProperties {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        format: tool.format.clone().into(),
                    },
                }))
            }
            // Hosted Responses tools (file_search, web_search, computer, mcp,
            // code_interpreter, image_generation, shell, apply_patch, etc.) have
            // no Chat Completions equivalent and are skipped at the call site.
            other => Err(TranslationError::InvalidPayload(format!(
                "OpenAI Responses tool `{}` cannot be translated to Chat Completions",
                tool_discriminant(other)
            ))),
        }
    }
}

impl From<responses::CustomToolParamFormat> for chat_request::CustomToolPropertiesFormat {
    fn from(value: responses::CustomToolParamFormat) -> Self {
        match value {
            responses::CustomToolParamFormat::Text => Self::Text,
            responses::CustomToolParamFormat::Grammar(grammar) => Self::Grammar {
                grammar: chat_request::CustomGrammarFormatParam {
                    definition: grammar.definition,
                    syntax: grammar.syntax.into(),
                },
            },
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
    tools: &Option<Vec<responses::Tool>>,
) -> TranslationResult<Option<Vec<chat::ChatCompletionTools>>> {
    let tools = match tools {
        None => return Ok(None),
        Some(tools) => tools,
    };

    let mut translated = Vec::new();
    for tool in tools {
        match chat::ChatCompletionTools::try_from(tool) {
            Ok(tool) => translated.push(tool),
            Err(error) => {
                tracing::trace!(
                    error = %error,
                    "skipping Responses tool without Chat Completions equivalent"
                );
            }
        }
    }
    if translated.is_empty() {
        Ok(None)
    } else {
        Ok(Some(translated))
    }
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
            responses::ToolChoiceParam::AllowedTools(allowed) => {
                // Responses `allowed_tools` already groups same-mode tools into a
                // single entry, but emit a Chat choice per entry to keep the
                // projection faithful. Chat itself supports a single
                // `allowed_tools` array of same-mode entries.
                Ok(Self::AllowedTools(
                    chat_request::ChatCompletionAllowedToolsChoice {
                        allowed_tools: vec![chat_request::ChatCompletionAllowedTools {
                            mode: allowed.mode.into(),
                            tools: allowed.tools.clone(),
                        }],
                    },
                ))
            }
            // Hosted tool choices (apply_patch, shell, mcp, file_search, etc.)
            // have no Chat representation.
            other => Err(TranslationError::InvalidPayload(format!(
                "OpenAI Responses tool_choice `{}` cannot be translated to Chat Completions",
                tool_choice_discriminant(other)
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

fn tool_discriminant(tool: &responses::Tool) -> &'static str {
    match tool {
        responses::Tool::Function(_) => "function",
        responses::Tool::FileSearch(_) => "file_search",
        responses::Tool::ComputerUsePreview(_) => "computer_use_preview",
        responses::Tool::WebSearch(_) => "web_search",
        responses::Tool::WebSearch20250826(_) => "web_search_20250826",
        responses::Tool::Mcp(_) => "mcp",
        responses::Tool::CodeInterpreter(_) => "code_interpreter",
        responses::Tool::ImageGeneration(_) => "image_generation",
        responses::Tool::LocalShell => "local_shell",
        responses::Tool::Shell(_) => "shell",
        responses::Tool::Custom(_) => "custom",
        responses::Tool::Computer(_) => "computer",
        responses::Tool::Namespace(_) => "namespace",
        responses::Tool::ToolSearch(_) => "tool_search",
        responses::Tool::WebSearchPreview(_) => "web_search_preview",
        responses::Tool::WebSearchPreview20250311(_) => "web_search_preview_20250311",
        responses::Tool::ApplyPatch => "apply_patch",
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
        responses::ToolChoiceParam::Hosted(hosted) => match hosted {
            responses::ToolChoiceTypes::FileSearch => "file_search",
            responses::ToolChoiceTypes::WebSearchPreview => "web_search_preview",
            responses::ToolChoiceTypes::Computer => "computer",
            responses::ToolChoiceTypes::ComputerUsePreview => "computer_use_preview",
            responses::ToolChoiceTypes::ComputerUse => "computer_use",
            responses::ToolChoiceTypes::WebSearchPreview20250311 => "web_search_preview_20250311",
            responses::ToolChoiceTypes::CodeInterpreter => "code_interpreter",
            responses::ToolChoiceTypes::ImageGeneration => "image_generation",
        },
        responses::ToolChoiceParam::Mode(_) => "mode",
    }
}
