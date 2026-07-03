use serde_json::Value;

use super::types::item_discriminant;
use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationResult;
use crate::translation::anthropic_messages::outbound::system_prompt_from_text_parts;
use crate::translation::anthropic_messages::outbound::{
    text_block, tool_use_block, url_image_block,
};
use crate::translation::text::join_text_parts;

pub(super) fn translate_messages(
    request: &responses::ResponseCreateParams,
) -> TranslationResult<(
    Option<anthropic::SystemPrompt>,
    Vec<anthropic::MessageParam>,
)> {
    let mut system_parts = Vec::new();
    if let Some(instructions) = request.instructions.as_deref()
        && !instructions.trim().is_empty()
    {
        system_parts.push(instructions.to_string());
    }

    let (input_system_parts, mut messages) = request
        .input
        .as_ref()
        .map(translate_input)
        .transpose()?
        .unwrap_or_default();
    system_parts.extend(input_system_parts);
    if messages.is_empty() {
        messages.push(anthropic::MessageParam {
            role: anthropic::MessageParamRole::User,
            content: anthropic::MessageParamContent::Text(String::new()),
        });
    }

    Ok((system_prompt_from_text_parts(system_parts), messages))
}

fn translate_input(
    input: &responses::InputParam,
) -> TranslationResult<(Vec<String>, Vec<anthropic::MessageParam>)> {
    match input {
        responses::InputParam::Text(text) => Ok((
            Vec::new(),
            vec![anthropic::MessageParam {
                role: anthropic::MessageParamRole::User,
                content: anthropic::MessageParamContent::Text(text.clone()),
            }],
        )),
        responses::InputParam::Items(items) => {
            let mut system_parts = Vec::new();
            let mut messages = Vec::new();

            for item in items {
                match item {
                    responses::InputItem::ItemReference(reference) => {
                        messages.push(reference.into())
                    }
                    responses::InputItem::EasyMessage(message) => match message.role {
                        responses::Role::System | responses::Role::Developer => {
                            if let Some(text) = extract_easy_text(&message.content) {
                                system_parts.push(text);
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
                                    if let Some(text) = extract_input_content_text(&input.content) {
                                        system_parts.push(text);
                                    }
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
                        responses::Item::FunctionCall(call) => append_message_content_block(
                            &mut messages,
                            anthropic::MessageParamRole::Assistant,
                            anthropic::ContentBlockParam::from(call),
                        ),
                        responses::Item::FunctionCallOutput(output) => {
                            append_message_content_block(
                                &mut messages,
                                anthropic::MessageParamRole::User,
                                anthropic::ContentBlockParam::ToolResult(
                                    anthropic::ToolResultBlockParam::try_from(output)?,
                                ),
                            )
                        }
                        responses::Item::CustomToolCall(call) => append_message_content_block(
                            &mut messages,
                            anthropic::MessageParamRole::Assistant,
                            anthropic::ContentBlockParam::from(call),
                        ),
                        responses::Item::CustomToolCallOutput(output) => {
                            append_message_content_block(
                                &mut messages,
                                anthropic::MessageParamRole::User,
                                anthropic::ContentBlockParam::ToolResult(
                                    anthropic::ToolResultBlockParam::from(output),
                                ),
                            )
                        }
                        other => {
                            tracing::trace!(
                                item_type = item_discriminant(other),
                                reason = "Responses item has no Anthropic Messages request representation"
                            );
                            messages.push(anthropic::MessageParam {
                                role: anthropic::MessageParamRole::User,
                                content: anthropic::MessageParamContent::Text(format!(
                                    "[OpenAI Responses item `{}` omitted during Anthropic translation]",
                                    item_discriminant(other)
                                )),
                            });
                        }
                    },
                }
            }

            Ok((system_parts, messages))
        }
    }
}

impl From<&responses::ItemReference> for anthropic::MessageParam {
    fn from(reference: &responses::ItemReference) -> Self {
        Self {
            role: anthropic::MessageParamRole::User,
            content: anthropic::MessageParamContent::Text(format!(
                "[OpenAI Responses item_reference `{}` omitted during Anthropic translation]",
                reference.id
            )),
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
                Self::Text(text_block(text.text.clone()))
            }
            responses::OutputMessageContent::Refusal(refusal) => {
                Self::Text(text_block(refusal.refusal.clone()))
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
                Ok(Self::Text(text_block(text.text.clone())))
            }
            responses::InputContent::InputImage(image) => match image.image_url.as_deref() {
                Some(url) => Ok(Self::Image(url_image_block(url))),
                None => Ok(Self::Text(text_block(
                    "[image omitted: only image_url is supported]".to_string(),
                ))),
            },
            responses::InputContent::InputFile(_) => Ok(Self::Text(text_block(
                "[file omitted during Anthropic translation]".to_string(),
            ))),
        }
    }
}

impl From<&responses::FunctionToolCall> for anthropic::ContentBlockParam {
    fn from(call: &responses::FunctionToolCall) -> Self {
        let input = serde_json::from_str::<Value>(&call.arguments)
            .unwrap_or_else(|_| Value::String(call.arguments.clone()));
        Self::ToolUse(tool_use_block(&call.call_id, &call.name, input))
    }
}

impl From<&responses::CustomToolCall> for anthropic::ContentBlockParam {
    fn from(call: &responses::CustomToolCall) -> Self {
        Self::ToolUse(tool_use_block(
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
                Ok(Self::Text(text_block(text.text.clone())))
            }
            responses::InputContent::InputImage(image) => match image.image_url.as_deref() {
                Some(url) => Ok(Self::Image(url_image_block(url))),
                None => Ok(Self::Text(text_block(
                    "[image omitted: only image_url is supported]".to_string(),
                ))),
            },
            responses::InputContent::InputFile(_) => Ok(Self::Text(text_block(
                "[file omitted during Anthropic translation]".to_string(),
            ))),
        }
    }
}

impl From<&responses::CustomToolCallOutput> for anthropic::ToolResultBlockParam {
    fn from(output: &responses::CustomToolCallOutput) -> Self {
        Self {
            tool_use_id: output.call_id.clone(),
            content: Some(match &output.output {
                responses::CustomToolCallOutputOutput::Text(text) => {
                    anthropic::ToolResultContentParam::Text(text.clone())
                }
                responses::CustomToolCallOutputOutput::List(values) => {
                    anthropic::ToolResultContentParam::Text(
                        serde_json::to_string(values).unwrap_or_else(|_| String::new()),
                    )
                }
            }),
            is_error: Some(false),
            cache_control: None,
        }
    }
}

fn append_message_content_block(
    messages: &mut Vec<anthropic::MessageParam>,
    role: anthropic::MessageParamRole,
    block: anthropic::ContentBlockParam,
) {
    let Some(last) = messages.last_mut() else {
        messages.push(anthropic::MessageParam {
            role,
            content: anthropic::MessageParamContent::Blocks(vec![block]),
        });
        return;
    };
    if last.role != role {
        messages.push(anthropic::MessageParam {
            role,
            content: anthropic::MessageParamContent::Blocks(vec![block]),
        });
        return;
    }

    match &mut last.content {
        anthropic::MessageParamContent::Blocks(content) => content.push(block),
        anthropic::MessageParamContent::Text(text) => {
            let previous_text = std::mem::take(text);
            last.content = anthropic::MessageParamContent::Blocks(vec![
                anthropic::ContentBlockParam::Text(text_block(previous_text)),
                block,
            ]);
        }
    }
}

fn extract_easy_text(content: &responses::EasyInputContent) -> Option<String> {
    match content {
        responses::EasyInputContent::Text(text) => Some(text.clone()),
        responses::EasyInputContent::ContentList(parts) => extract_input_content_text(parts),
    }
}

fn extract_input_content_text(parts: &[responses::InputContent]) -> Option<String> {
    join_text_parts(
        parts
            .iter()
            .filter_map(|part| match part {
                responses::InputContent::InputText(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect(),
    )
}
