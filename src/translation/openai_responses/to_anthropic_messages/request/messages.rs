use serde_json::Value;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::anthropic_messages::outbound::{
    content_block_message, document_source_from_file_data, document_source_from_url,
    image_block_from_url, merge_adjacent_tool_messages, text_block_param, tool_use_block_param,
    typed_text_block,
};

use crate::translation::{TranslationError, TranslationResult};

pub(super) fn translate_messages(
    instructions: Option<&str>,
    input: Option<&responses::InputParam>,
) -> TranslationResult<(
    Option<anthropic::SystemPrompt>,
    Vec<anthropic::MessageParam>,
)> {
    let mut system_blocks = Vec::new();
    if let Some(instructions) = instructions
        && !instructions.trim().is_empty()
    {
        system_blocks.push(typed_text_block(instructions.to_string()));
    }

    let (input_system_blocks, messages) =
        input.map(translate_input).transpose()?.unwrap_or_default();
    system_blocks.extend(input_system_blocks);
    if messages.is_empty() {
        return Err(TranslationError::InvalidPayload(
            "OpenAI Responses request must contain at least one user or assistant input item to translate to Anthropic Messages"
                .to_string(),
        ));
    }

    Ok((system_prompt_from_text_blocks(system_blocks), messages))
}

fn translate_input(
    input: &responses::InputParam,
) -> TranslationResult<(
    Vec<anthropic::TypedTextBlockParam>,
    Vec<anthropic::MessageParam>,
)> {
    match input {
        responses::InputParam::Text(text) => Ok((
            Vec::new(),
            vec![anthropic::MessageParam {
                role: anthropic::MessageParamRole::User,
                content: anthropic::MessageParamContent::Text(text.clone()),
            }],
        )),
        responses::InputParam::Items(items) => {
            let mut system_blocks = Vec::new();
            let mut messages = Vec::new();

            for item in items {
                match item {
                    responses::InputItem::ItemReference(reference) => {
                        return Err(TranslationError::InvalidPayload(format!(
                            "OpenAI Responses item_reference `{}` cannot be translated to Anthropic Messages because the referenced item content is not available",
                            reference.id
                        )));
                    }
                    responses::InputItem::EasyMessage(message) => match message.role {
                        responses::Role::System | responses::Role::Developer => {
                            match &message.content {
                                responses::EasyInputContent::Text(text)
                                    if !text.trim().is_empty() =>
                                {
                                    system_blocks.push(typed_text_block(text.clone()));
                                }
                                responses::EasyInputContent::ContentList(parts) => {
                                    system_blocks.extend(system_blocks_from_input_content(parts)?);
                                }
                                responses::EasyInputContent::Text(_) => {}
                            }
                        }
                        responses::Role::Assistant => messages.push(anthropic::MessageParam {
                            role: anthropic::MessageParamRole::Assistant,
                            content: anthropic::MessageParamContent::try_from(&message.content)?,
                        }),
                        responses::Role::User => messages.push(anthropic::MessageParam {
                            role: anthropic::MessageParamRole::User,
                            content: anthropic::MessageParamContent::try_from(&message.content)?,
                        }),
                    },
                    responses::InputItem::Item(item) => match item {
                        responses::Item::Message(responses::MessageItem::Input(input)) => {
                            match input.role {
                                responses::InputRole::System | responses::InputRole::Developer => {
                                    system_blocks
                                        .extend(system_blocks_from_input_content(&input.content)?);
                                }
                                responses::InputRole::User => {
                                    messages.push(anthropic::MessageParam {
                                        role: anthropic::MessageParamRole::User,
                                        content: anthropic::MessageParamContent::Blocks(
                                            translate_input_content_list(&input.content)?,
                                        ),
                                    });
                                }
                            }
                        }
                        responses::Item::Message(responses::MessageItem::Output(output)) => {
                            messages.push(output.into());
                        }
                        responses::Item::FunctionCall(call) => {
                            messages.push(content_block_message(
                                anthropic::MessageParamRole::Assistant,
                                anthropic::ContentBlockParam::from(call),
                            ));
                        }
                        responses::Item::FunctionCallOutput(output) => {
                            messages.push(content_block_message(
                                anthropic::MessageParamRole::User,
                                anthropic::ContentBlockParam::ToolResult(
                                    anthropic::ToolResultBlockParam::try_from(output)?,
                                ),
                            ));
                        }
                        responses::Item::CustomToolCall(call) => {
                            messages.push(content_block_message(
                                anthropic::MessageParamRole::Assistant,
                                anthropic::ContentBlockParam::from(call),
                            ));
                        }
                        responses::Item::CustomToolCallOutput(output) => {
                            messages.push(content_block_message(
                                anthropic::MessageParamRole::User,
                                anthropic::ContentBlockParam::ToolResult(
                                    anthropic::ToolResultBlockParam::try_from(output)?,
                                ),
                            ));
                        }
                        responses::Item::Reasoning(reasoning) => {
                            if let Some(data) = &reasoning.encrypted_content {
                                messages.push(content_block_message(
                                    anthropic::MessageParamRole::Assistant,
                                    anthropic::ContentBlockParam::RedactedThinking(
                                        anthropic::RedactedThinkingBlockParam {
                                            data: data.clone(),
                                        },
                                    ),
                                ));
                            }

                            let has_content = reasoning
                                .content
                                .as_ref()
                                .is_some_and(|content| !content.is_empty());
                            if !reasoning.summary.is_empty() || has_content {
                                tracing::trace!(
                                    has_summary = !reasoning.summary.is_empty(),
                                    has_content,
                                    reason = "OpenAI Responses reasoning history has no Anthropic thinking signature",
                                    "skipping unsigned OpenAI Responses reasoning history during Anthropic Messages request translation"
                                );
                            }
                        }
                        other => {
                            return Err(TranslationError::InvalidPayload(format!(
                                "OpenAI Responses item `{}` cannot be translated to Anthropic Messages request content",
                                other.as_ref()
                            )));
                        }
                    },
                }
            }

            Ok((system_blocks, merge_adjacent_tool_messages(messages)))
        }
    }
}

impl From<&responses::OutputMessage> for anthropic::MessageParam {
    fn from(message: &responses::OutputMessage) -> Self {
        Self {
            role: anthropic::MessageParamRole::Assistant,
            content: anthropic::MessageParamContent::Blocks(
                message.content.iter().map(Into::into).collect(),
            ),
        }
    }
}

impl From<&responses::OutputMessageContent> for anthropic::ContentBlockParam {
    fn from(content: &responses::OutputMessageContent) -> Self {
        match content {
            responses::OutputMessageContent::OutputText(text) => {
                Self::Text(text_block_param(text.text.clone()))
            }
            responses::OutputMessageContent::Refusal(refusal) => {
                Self::Text(text_block_param(refusal.refusal.clone()))
            }
        }
    }
}

impl TryFrom<&responses::EasyInputContent> for anthropic::MessageParamContent {
    type Error = crate::translation::TranslationError;

    fn try_from(content: &responses::EasyInputContent) -> TranslationResult<Self> {
        match content {
            responses::EasyInputContent::Text(text) => Ok(Self::Text(text.clone())),
            responses::EasyInputContent::ContentList(parts) => {
                Ok(Self::Blocks(translate_input_content_list(parts)?))
            }
        }
    }
}

fn translate_input_content_list(
    parts: &[responses::InputContent],
) -> TranslationResult<Vec<anthropic::ContentBlockParam>> {
    parts
        .iter()
        .map(anthropic::ContentBlockParam::try_from)
        .collect()
}

impl TryFrom<&responses::InputContent> for anthropic::ContentBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(part: &responses::InputContent) -> TranslationResult<Self> {
        match part {
            responses::InputContent::InputText(text) => {
                Ok(Self::Text(text_block_param(text.text.clone())))
            }
            responses::InputContent::InputImage(image) => Ok(Self::Image(image.try_into()?)),
            responses::InputContent::InputFile(file) => Ok(Self::Document(file.try_into()?)),
        }
    }
}

impl TryFrom<&responses::InputImageContent> for anthropic::ImageBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(image: &responses::InputImageContent) -> TranslationResult<Self> {
        let Some(url) = image.image_url.as_deref() else {
            return Err(TranslationError::InvalidPayload(
                if image.file_id.is_some() {
                    "OpenAI Responses input_image.file_id cannot be translated to Anthropic Messages image content; file IDs are provider-scoped"
                } else {
                    "OpenAI Responses input_image must include image_url as either a URL or data:image/<type>;base64,<data> value to translate to Anthropic Messages"
                }
                .to_string(),
            ));
        };

        image_block_from_url(url)
    }
}

impl TryFrom<&responses::InputFileContent> for anthropic::DocumentBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(file: &responses::InputFileContent) -> TranslationResult<Self> {
        let source = if let Some(data) = file.file_data.as_deref() {
            document_source_from_file_data(data)?
        } else if let Some(url) = file.file_url.as_deref() {
            document_source_from_url(url)?
        } else if file.file_id.is_some() {
            return Err(TranslationError::InvalidPayload(
                "OpenAI Responses input_file.file_id cannot be translated to Anthropic Messages document content; file IDs are provider-scoped"
                    .to_string(),
            ));
        } else {
            return Err(TranslationError::InvalidPayload(
                "OpenAI Responses input_file must include file_data or file_url to translate to Anthropic Messages"
                    .to_string(),
            ));
        };

        Ok(anthropic::DocumentBlockParam {
            source,
            title: file.filename.clone(),
            cache_control: None,
            citations: None,
            context: None,
        })
    }
}

impl From<&responses::FunctionToolCall> for anthropic::ContentBlockParam {
    fn from(call: &responses::FunctionToolCall) -> Self {
        let input = serde_json::from_str::<Value>(&call.arguments)
            .unwrap_or_else(|_| Value::String(call.arguments.clone()));
        Self::ToolUse(tool_use_block_param(&call.call_id, &call.name, input))
    }
}

impl From<&responses::CustomToolCall> for anthropic::ContentBlockParam {
    fn from(call: &responses::CustomToolCall) -> Self {
        Self::ToolUse(tool_use_block_param(
            &call.call_id,
            &call.name,
            Value::String(call.input.clone()),
        ))
    }
}

impl TryFrom<&responses::FunctionCallOutputItemParam> for anthropic::ToolResultBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(output: &responses::FunctionCallOutputItemParam) -> TranslationResult<Self> {
        Ok(Self {
            tool_use_id: output.call_id.clone(),
            content: Some(anthropic::ToolResultContentParam::try_from(&output.output)?),
            is_error: Some(false),
            cache_control: None,
        })
    }
}

impl TryFrom<&responses::FunctionCallOutput> for anthropic::ToolResultContentParam {
    type Error = crate::translation::TranslationError;

    fn try_from(output: &responses::FunctionCallOutput) -> TranslationResult<Self> {
        match output {
            responses::FunctionCallOutput::Text(text) => Ok(Self::Text(text.clone())),
            responses::FunctionCallOutput::Content(parts) => parts
                .iter()
                .map(anthropic::ToolResultContentBlockParam::try_from)
                .collect::<TranslationResult<Vec<_>>>()
                .map(Self::Blocks),
        }
    }
}

impl TryFrom<&responses::InputContent> for anthropic::ToolResultContentBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(part: &responses::InputContent) -> TranslationResult<Self> {
        match part {
            responses::InputContent::InputText(text) => {
                Ok(Self::Text(text_block_param(text.text.clone())))
            }
            responses::InputContent::InputImage(image) => Ok(Self::Image(image.try_into()?)),
            responses::InputContent::InputFile(file) => Ok(Self::Document(file.try_into()?)),
        }
    }
}

impl TryFrom<&responses::CustomToolCallOutput> for anthropic::ToolResultBlockParam {
    type Error = crate::translation::TranslationError;

    fn try_from(output: &responses::CustomToolCallOutput) -> TranslationResult<Self> {
        Ok(Self {
            tool_use_id: output.call_id.clone(),
            content: Some(match &output.output {
                responses::CustomToolCallOutputOutput::Text(text) => {
                    anthropic::ToolResultContentParam::Text(text.clone())
                }
                responses::CustomToolCallOutputOutput::List(parts) => {
                    anthropic::ToolResultContentParam::Blocks(
                        parts
                            .iter()
                            .map(anthropic::ToolResultContentBlockParam::try_from)
                            .collect::<TranslationResult<Vec<_>>>()?,
                    )
                }
            }),
            is_error: Some(false),
            cache_control: None,
        })
    }
}

fn system_prompt_from_text_blocks(
    blocks: Vec<anthropic::TypedTextBlockParam>,
) -> Option<anthropic::SystemPrompt> {
    let blocks: Vec<_> = blocks
        .into_iter()
        .filter(|block| !block.text.trim().is_empty())
        .collect();

    match blocks.len() {
        0 => None,
        1 => blocks
            .into_iter()
            .next()
            .map(|block| anthropic::SystemPrompt::Text(block.text)),
        _ => Some(anthropic::SystemPrompt::Blocks(blocks)),
    }
}

fn system_blocks_from_input_content(
    parts: &[responses::InputContent],
) -> TranslationResult<Vec<anthropic::TypedTextBlockParam>> {
    let mut blocks = Vec::new();
    for part in parts {
        match part {
            responses::InputContent::InputText(text) if !text.text.trim().is_empty() => {
                blocks.push(typed_text_block(text.text.clone()));
            }
            responses::InputContent::InputText(_) => {}
            responses::InputContent::InputImage(_) => {
                return Err(TranslationError::InvalidPayload(
                    "OpenAI Responses system/developer message content cannot include input_image when translating to Anthropic Messages system prompt"
                        .to_string(),
                ));
            }
            responses::InputContent::InputFile(_) => {
                return Err(TranslationError::InvalidPayload(
                    "OpenAI Responses system/developer message content cannot include input_file when translating to Anthropic Messages system prompt"
                        .to_string(),
                ));
            }
        }
    }
    Ok(blocks)
}
