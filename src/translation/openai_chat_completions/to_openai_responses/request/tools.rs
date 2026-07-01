use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire::{
    CustomToolPropertiesFormat, GrammarSyntax, ToolChoiceAllowedMode,
};
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&chat::ChatCompletionTools> for responses::Tool {
    type Error = TranslationError;

    fn try_from(value: &chat::ChatCompletionTools) -> TranslationResult<Self> {
        match value {
            chat::ChatCompletionTools::Function(tool) => {
                Ok(responses::Tool::Function(responses::FunctionTool {
                    name: tool.function.name.clone(),
                    parameters: tool.function.parameters.clone(),
                    strict: tool.function.strict,
                    description: tool.function.description.clone(),
                    defer_loading: None,
                }))
            }
            chat::ChatCompletionTools::Custom(tool) => {
                Ok(responses::Tool::Custom(responses::CustomToolParam {
                    name: tool.custom.name.clone(),
                    description: tool.custom.description.clone(),
                    format: tool.custom.format.clone().into(),
                    defer_loading: None,
                }))
            }
        }
    }
}

impl From<CustomToolPropertiesFormat> for responses::CustomToolParamFormat {
    fn from(value: CustomToolPropertiesFormat) -> Self {
        match value {
            CustomToolPropertiesFormat::Text => Self::Text,
            CustomToolPropertiesFormat::Grammar { grammar } => {
                Self::Grammar(responses::CustomGrammarFormatParam {
                    definition: grammar.definition,
                    syntax: grammar.syntax.into(),
                })
            }
        }
    }
}

impl From<GrammarSyntax> for responses::GrammarSyntax {
    fn from(value: GrammarSyntax) -> Self {
        match value {
            GrammarSyntax::Lark => Self::Lark,
            GrammarSyntax::Regex => Self::Regex,
        }
    }
}

impl From<ToolChoiceAllowedMode> for responses::ToolChoiceAllowedMode {
    fn from(value: ToolChoiceAllowedMode) -> Self {
        match value {
            ToolChoiceAllowedMode::Auto => Self::Auto,
            ToolChoiceAllowedMode::Required => Self::Required,
        }
    }
}

impl TryFrom<&chat::ChatCompletionToolChoiceOption> for responses::ToolChoiceParam {
    type Error = TranslationError;

    fn try_from(value: &chat::ChatCompletionToolChoiceOption) -> TranslationResult<Self> {
        match value {
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::None) => Ok(
                responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::None),
            ),
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Auto) => Ok(
                responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Auto),
            ),
            chat::ChatCompletionToolChoiceOption::Mode(chat::ToolChoiceOptions::Required) => Ok(
                responses::ToolChoiceParam::Mode(responses::ToolChoiceOptions::Required),
            ),
            chat::ChatCompletionToolChoiceOption::Function(choice) => Ok(
                responses::ToolChoiceParam::Function(responses::ToolChoiceFunction {
                    name: choice.function.name.clone(),
                }),
            ),
            chat::ChatCompletionToolChoiceOption::Custom(choice) => Ok(
                responses::ToolChoiceParam::Custom(responses::ToolChoiceCustom {
                    name: choice.custom.name.clone(),
                }),
            ),
            chat::ChatCompletionToolChoiceOption::AllowedTools(choice) => {
                let first = choice.allowed_tools.first().ok_or_else(|| {
                    TranslationError::InvalidPayload(
                        "Chat Completions allowed_tools tool_choice must contain at least one entry to translate to OpenAI Responses"
                            .to_string(),
                    )
                })?;

                let mut tools = Vec::new();
                for allowed_tools in &choice.allowed_tools {
                    if allowed_tools.mode != first.mode {
                        return Err(TranslationError::InvalidPayload(
                            "Chat Completions allowed_tools tool_choice cannot mix modes when translating to OpenAI Responses"
                                .to_string(),
                        ));
                    }
                    tools.extend(allowed_tools.tools.iter().cloned());
                }

                Ok(responses::ToolChoiceParam::AllowedTools(
                    responses::ToolChoiceAllowed {
                        mode: first.mode.into(),
                        tools,
                    },
                ))
            }
        }
    }
}
