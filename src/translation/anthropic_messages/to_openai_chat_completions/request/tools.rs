use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::translation::{TranslationError, TranslationResult};

use super::types::non_empty;

pub(super) struct ChatToolConfig {
    pub(super) tools: Option<Vec<chat::ChatCompletionTools>>,
    pub(super) tool_choice: Option<chat::ChatCompletionToolChoiceOption>,
    pub(super) parallel_tool_calls: Option<bool>,
}

pub(super) fn chat_tool_config(
    tools: Option<Vec<anthropic::ToolUnion>>,
    tool_choice: Option<anthropic::ToolChoice>,
) -> TranslationResult<ChatToolConfig> {
    let tools = tools
        .map(|tools| {
            tools
                .into_iter()
                .map(TryInto::try_into)
                .collect::<TranslationResult<Vec<_>>>()
        })
        .transpose()?
        .and_then(non_empty);
    validate_tool_choice(tool_choice.as_ref(), &tools)?;
    let parallel_tool_calls = parallel_tool_calls(tool_choice.as_ref());
    let tool_choice = tool_choice.map(Into::into);

    Ok(ChatToolConfig {
        tools,
        tool_choice,
        parallel_tool_calls,
    })
}

fn validate_tool_choice(
    choice: Option<&anthropic::ToolChoice>,
    tools: &Option<Vec<chat::ChatCompletionTools>>,
) -> TranslationResult<()> {
    let Some(anthropic::ToolChoice::Tool(choice)) = choice else {
        return Ok(());
    };

    let exists = tools.as_ref().is_some_and(|tools| {
        tools.iter().any(|tool| match tool {
            chat::ChatCompletionTools::Function(tool) => tool.function.name == choice.name,
            chat::ChatCompletionTools::Custom(tool) => tool.custom.name == choice.name,
        })
    });
    if !exists {
        return Err(TranslationError::InvalidPayload(format!(
            "Anthropic tool_choice references tool `{}`, but no translated Chat Completions tool with that name exists",
            choice.name
        )));
    }

    Ok(())
}

fn parallel_tool_calls(choice: Option<&anthropic::ToolChoice>) -> Option<bool> {
    let disable_parallel_tool_use = match choice? {
        anthropic::ToolChoice::Auto(choice) => choice.disable_parallel_tool_use,
        anthropic::ToolChoice::Any(choice) => choice.disable_parallel_tool_use,
        anthropic::ToolChoice::Tool(choice) => choice.disable_parallel_tool_use,
        anthropic::ToolChoice::None(_) => None,
    };

    disable_parallel_tool_use.map(|disable| !disable)
}

impl TryFrom<anthropic::ToolUnion> for chat::ChatCompletionTools {
    type Error = TranslationError;

    fn try_from(tool: anthropic::ToolUnion) -> TranslationResult<Self> {
        match tool {
            anthropic::ToolUnion::Custom(tool) => {
                Ok(Self::Function(chat_request::ChatCompletionTool {
                    function: chat_request::FunctionObject {
                        name: tool.name,
                        description: tool.description,
                        parameters: Some(serde_json::to_value(tool.input_schema)?),
                        strict: tool.strict,
                    },
                }))
            }
            other => Err(TranslationError::InvalidPayload(format!(
                "Anthropic tool `{}` cannot be translated to Chat Completions tools",
                other.as_ref()
            ))),
        }
    }
}

impl From<anthropic::ToolChoice> for chat::ChatCompletionToolChoiceOption {
    fn from(choice: anthropic::ToolChoice) -> Self {
        match choice {
            anthropic::ToolChoice::Auto(_) => Self::Mode(chat::ToolChoiceOptions::Auto),
            anthropic::ToolChoice::Any(_) => Self::Mode(chat::ToolChoiceOptions::Required),
            anthropic::ToolChoice::None(_) => Self::Mode(chat::ToolChoiceOptions::None),
            anthropic::ToolChoice::Tool(choice) => {
                Self::Function(chat_request::ChatCompletionNamedToolChoice {
                    function: chat_request::FunctionName { name: choice.name },
                })
            }
        }
    }
}
