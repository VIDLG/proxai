use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationError, TranslationResult};

use super::types::non_empty;

pub(super) struct ResponsesToolConfig {
    pub(super) tools: Option<Vec<responses::Tool>>,
    pub(super) tool_choice: Option<responses::ToolChoiceParam>,
    pub(super) parallel_tool_calls: Option<bool>,
}

pub(super) fn responses_tool_config(
    tools: Option<Vec<anthropic::ToolUnion>>,
    tool_choice: Option<anthropic::ToolChoice>,
) -> TranslationResult<ResponsesToolConfig> {
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

    Ok(ResponsesToolConfig {
        tools,
        tool_choice,
        parallel_tool_calls,
    })
}

fn validate_tool_choice(
    choice: Option<&anthropic::ToolChoice>,
    tools: &Option<Vec<responses::Tool>>,
) -> TranslationResult<()> {
    let Some(anthropic::ToolChoice::Tool(choice)) = choice else {
        return Ok(());
    };

    let exists = tools.as_ref().is_some_and(|tools| {
        tools.iter().any(|tool| match tool {
            responses::Tool::Function(tool) => tool.name == choice.name,
            _ => false,
        })
    });
    if !exists {
        return Err(TranslationError::InvalidPayload(format!(
            "Anthropic tool_choice references tool `{}`, but no translated OpenAI Responses tool with that name exists",
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

impl TryFrom<anthropic::ToolUnion> for responses::Tool {
    type Error = TranslationError;

    fn try_from(tool: anthropic::ToolUnion) -> TranslationResult<Self> {
        match tool {
            anthropic::ToolUnion::Custom(tool) => Ok(Self::Function(responses::FunctionTool {
                name: tool.name,
                parameters: Some(serde_json::to_value(tool.input_schema)?),
                strict: tool.strict,
                description: tool.description,
                defer_loading: tool.defer_loading,
            })),
            other => Err(TranslationError::InvalidPayload(format!(
                "Anthropic tool `{}` cannot be translated to OpenAI Responses tools",
                other.as_ref()
            ))),
        }
    }
}

impl From<anthropic::ToolChoice> for responses::ToolChoiceParam {
    fn from(choice: anthropic::ToolChoice) -> Self {
        match choice {
            anthropic::ToolChoice::Auto(_) => Self::Mode(responses::ToolChoiceOptions::Auto),
            anthropic::ToolChoice::Any(_) => Self::Mode(responses::ToolChoiceOptions::Required),
            anthropic::ToolChoice::None(_) => Self::Mode(responses::ToolChoiceOptions::None),
            anthropic::ToolChoice::Tool(choice) => {
                Self::Function(responses::ToolChoiceFunction { name: choice.name })
            }
        }
    }
}
