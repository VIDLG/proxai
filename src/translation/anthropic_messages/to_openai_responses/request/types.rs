//! Reusable `From` impls for Anthropic → Responses type conversions.

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::openai_responses::outbound::{
    input_file_data, input_file_url, input_image_url, input_text, pending_function_tool_call,
};
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn non_empty<T>(items: Vec<T>) -> Option<Vec<T>> {
    if items.is_empty() { None } else { Some(items) }
}

impl TryFrom<anthropic::OutputFormat> for responses::ResponseTextParam {
    type Error = TranslationError;

    fn try_from(format: anthropic::OutputFormat) -> TranslationResult<Self> {
        match format {
            anthropic::OutputFormat::JsonSchema(schema) => Ok(Self {
                format: responses::TextResponseFormatConfiguration::JsonSchema(
                    responses::ResponseFormatJsonSchema {
                        description: None,
                        name: "anthropic_json_schema".to_string(),
                        schema: Some(schema.schema),
                        strict: None,
                    },
                ),
                verbosity: None,
            }),
        }
    }
}

impl From<anthropic::RequestServiceTier> for responses::ServiceTier {
    fn from(tier: anthropic::RequestServiceTier) -> Self {
        match tier {
            anthropic::RequestServiceTier::Auto => Self::Auto,
            anthropic::RequestServiceTier::StandardOnly => Self::Default,
        }
    }
}

impl From<&anthropic::SystemPrompt> for String {
    fn from(system: &anthropic::SystemPrompt) -> Self {
        match system {
            anthropic::SystemPrompt::Text(text) => text.clone(),
            anthropic::SystemPrompt::Blocks(blocks) => blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl From<anthropic::TextBlockParam> for responses::InputContent {
    fn from(block: anthropic::TextBlockParam) -> Self {
        input_text(block.text)
    }
}

impl From<anthropic::ImageBlockParam> for responses::InputContent {
    fn from(block: anthropic::ImageBlockParam) -> Self {
        let image_url = match block.source {
            anthropic::ImageBlockSource::Base64(source) => {
                format!("data:{};base64,{}", source.media_type.as_ref(), source.data)
            }
            anthropic::ImageBlockSource::Url(source) => source.url,
        };
        input_image_url(image_url)
    }
}

impl From<anthropic::ContentBlockSourceContent> for responses::InputContent {
    fn from(block: anthropic::ContentBlockSourceContent) -> Self {
        match block {
            anthropic::ContentBlockSourceContent::Text(block) => block.into(),
            anthropic::ContentBlockSourceContent::Image(block) => block.into(),
        }
    }
}

impl TryFrom<anthropic::ToolUseBlockParam> for responses::Item {
    type Error = TranslationError;

    fn try_from(block: anthropic::ToolUseBlockParam) -> TranslationResult<Self> {
        let arguments = serde_json::to_string(&block.input)?;
        Ok(Self::FunctionCall(pending_function_tool_call(
            &block.id,
            &block.name,
            arguments,
        )))
    }
}

impl TryFrom<anthropic::ToolResultBlockParam> for responses::Item {
    type Error = crate::translation::TranslationError;

    fn try_from(block: anthropic::ToolResultBlockParam) -> TranslationResult<Self> {
        let output = match block.content {
            Some(anthropic::ToolResultContentParam::Text(text)) => {
                responses::FunctionCallOutput::Text(text)
            }
            Some(anthropic::ToolResultContentParam::Blocks(blocks)) => {
                let parts: Vec<responses::InputContent> = blocks
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()?;
                responses::FunctionCallOutput::Content(parts)
            }
            None => responses::FunctionCallOutput::Content(Vec::new()),
        };
        let status = match block.is_error {
            Some(true) => responses::OutputStatus::Incomplete,
            _ => responses::OutputStatus::Completed,
        };
        Ok(Self::FunctionCallOutput(
            responses::FunctionCallOutputItemParam {
                call_id: block.tool_use_id,
                output,
                id: None, // No Anthropic equivalent; Responses item id is server-assigned.
                status: Some(status),
            },
        ))
    }
}

impl TryFrom<anthropic::DocumentBlockParam> for responses::InputContent {
    type Error = crate::translation::TranslationError;

    fn try_from(doc_block: anthropic::DocumentBlockParam) -> TranslationResult<Self> {
        match doc_block.source {
            anthropic::DocumentBlockParamSource::Base64(source) => {
                let data = format!("data:{};base64,{}", source.media_type.as_ref(), source.data);
                Ok(input_file_data(data, doc_block.title))
            }
            anthropic::DocumentBlockParamSource::Url(source) => {
                Ok(input_file_url(source.url, doc_block.title))
            }
            anthropic::DocumentBlockParamSource::PlainText(source) => {
                let data = format!("data:{};base64,{}", source.media_type.as_ref(), source.data);
                Ok(input_file_data(data, doc_block.title))
            }
            anthropic::DocumentBlockParamSource::Content(content) => match content.content {
                anthropic::ContentBlockSourceContentUnion::Text(text) => Ok(input_file_data(
                    format!("data:text/plain,{}", text),
                    doc_block.title,
                )),
                anthropic::ContentBlockSourceContentUnion::Blocks(blocks) => {
                    if blocks.len() == 1 {
                        let content: responses::InputContent =
                            blocks.into_iter().next().unwrap().into();
                        Ok(content)
                    } else {
                        Err(crate::translation::TranslationError::InvalidPayload(
                            "Anthropic document content with multiple blocks is not \
                             supported in OpenAI Responses input"
                                .to_string(),
                        ))
                    }
                }
            },
        }
    }
}

impl TryFrom<anthropic::ToolResultContentBlockParam> for responses::InputContent {
    type Error = crate::translation::TranslationError;

    fn try_from(block: anthropic::ToolResultContentBlockParam) -> TranslationResult<Self> {
        match block {
            anthropic::ToolResultContentBlockParam::Text(block) => Ok(block.into()),
            anthropic::ToolResultContentBlockParam::Image(block) => Ok(block.into()),
            anthropic::ToolResultContentBlockParam::Document(doc_block) => doc_block.try_into(),
            other => {
                let name = match other {
                    anthropic::ToolResultContentBlockParam::SearchResult(_) => "search_result",
                    anthropic::ToolResultContentBlockParam::ToolReference(_) => "tool_reference",
                    _ => "unknown",
                };
                Err(crate::translation::TranslationError::InvalidPayload(
                    format!(
                        "Anthropic tool result content block type `{}` is not supported \
                         in OpenAI Responses input",
                        name,
                    ),
                ))
            }
        }
    }
}

impl From<anthropic::MessageParamRole> for responses::Role {
    fn from(role: anthropic::MessageParamRole) -> Self {
        match role {
            anthropic::MessageParamRole::Assistant => Self::Assistant,
            anthropic::MessageParamRole::User => Self::User,
            anthropic::MessageParamRole::System => Self::System,
        }
    }
}
