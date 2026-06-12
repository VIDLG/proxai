use serde_json::Value;

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::{TranslationError, TranslationResult};

pub(super) struct AnthropicMessages {
    pub system: Option<anthropic::SystemPrompt>,
    pub messages: Vec<anthropic::MessageParam>,
}

impl TryFrom<&[chat::ChatCompletionRequestMessage]> for AnthropicMessages {
    type Error = TranslationError;

    fn try_from(chat_messages: &[chat::ChatCompletionRequestMessage]) -> TranslationResult<Self> {
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();

        for message in chat_messages {
            match message {
                chat::ChatCompletionRequestMessage::Developer(message) => {
                    // Anthropic Messages has no developer role, so Chat developer
                    // and system instructions are folded into the top-level system prompt.
                    system_parts.extend(developer_text_parts(&message.content));
                }
                chat::ChatCompletionRequestMessage::System(message) => {
                    system_parts.extend(system_text_parts(&message.content));
                }
                chat::ChatCompletionRequestMessage::User(message) => {
                    messages.push(anthropic::MessageParam {
                        role: anthropic::Role::User,
                        content: anthropic::MessageParamContent::try_from(&message.content)?,
                    });
                }
                chat::ChatCompletionRequestMessage::Assistant(message) => {
                    messages.push(anthropic::MessageParam {
                        role: anthropic::Role::Assistant,
                        content: anthropic::MessageParamContent::try_from(message)?,
                    });
                }
                chat::ChatCompletionRequestMessage::Tool(message) => {
                    messages.push(anthropic::MessageParam {
                        role: anthropic::Role::User,
                        content: anthropic::MessageParamContent::Blocks(vec![
                            anthropic::ContentBlockParam::ToolResult(message.into()),
                        ]),
                    });
                }
                chat::ChatCompletionRequestMessage::Function(_) => {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions legacy function messages cannot be translated to Anthropic Messages because they do not carry a tool_call_id"
                            .to_string(),
                    ));
                }
            }
        }

        if messages.is_empty() {
            return Err(TranslationError::InvalidPayload(
                "Chat Completions request must contain at least one non-system message to translate to Anthropic Messages"
                    .to_string(),
            ));
        }

        Ok(Self {
            system: anthropic_system_prompt(system_parts),
            messages,
        })
    }
}

fn developer_text_parts(
    content: &chat::ChatCompletionRequestDeveloperMessageContent,
) -> Vec<String> {
    match content {
        chat::ChatCompletionRequestDeveloperMessageContent::Text(text) => vec![text.clone()],
        chat::ChatCompletionRequestDeveloperMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestDeveloperMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            })
            .collect(),
    }
}

fn system_text_parts(content: &chat::ChatCompletionRequestSystemMessageContent) -> Vec<String> {
    match content {
        chat::ChatCompletionRequestSystemMessageContent::Text(text) => vec![text.clone()],
        chat::ChatCompletionRequestSystemMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestSystemMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            })
            .collect(),
    }
}

fn anthropic_system_prompt(text_parts: Vec<String>) -> Option<anthropic::SystemPrompt> {
    let mut text_parts = text_parts
        .into_iter()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    match text_parts.len() {
        0 => None,
        1 => text_parts.pop().map(anthropic::SystemPrompt::Text),
        _ => Some(anthropic::SystemPrompt::Blocks(
            text_parts
                .into_iter()
                .map(|text| anthropic::TypedTextBlockParam {
                    type_: anthropic::TextBlockType::Text,
                    text,
                    // Chat system/developer text has no Anthropic cache-control or
                    // citation metadata to preserve.
                    cache_control: None,
                    citations: None,
                })
                .collect(),
        )),
    }
}

impl TryFrom<&chat::ChatCompletionRequestUserMessageContent> for anthropic::MessageParamContent {
    type Error = TranslationError;

    fn try_from(
        content: &chat::ChatCompletionRequestUserMessageContent,
    ) -> TranslationResult<Self> {
        match content {
            chat::ChatCompletionRequestUserMessageContent::Text(text) if text.is_empty() => Err(
                TranslationError::InvalidPayload(
                    "Chat Completions user message without content cannot be translated to Anthropic Messages"
                        .to_string(),
                ),
            ),
            chat::ChatCompletionRequestUserMessageContent::Text(text) => {
                Ok(anthropic::MessageParamContent::Text(text.clone()))
            }
            chat::ChatCompletionRequestUserMessageContent::Array(parts) => {
                let blocks = parts
                    .iter()
                    .map(anthropic::ContentBlockParam::try_from)
                    .collect::<TranslationResult<Vec<_>>>()?;
                if blocks.is_empty() {
                    Err(TranslationError::InvalidPayload(
                        "Chat Completions user message without content cannot be translated to Anthropic Messages"
                            .to_string(),
                    ))
                } else {
                    Ok(anthropic::MessageParamContent::Blocks(blocks))
                }
            }
        }
    }
}

impl TryFrom<&chat::ChatCompletionRequestUserMessageContentPart> for anthropic::ContentBlockParam {
    type Error = TranslationError;

    fn try_from(
        part: &chat::ChatCompletionRequestUserMessageContentPart,
    ) -> TranslationResult<Self> {
        match part {
            chat::ChatCompletionRequestUserMessageContentPart::Text(part) => {
                Ok(anthropic::ContentBlockParam::Text(anthropic::TextBlockParam {
                    text: part.text.clone(),
                    cache_control: None,
                    citations: None,
                }))
            }
            chat::ChatCompletionRequestUserMessageContentPart::ImageUrl(part) => Ok(
                anthropic::ContentBlockParam::Image(anthropic::ImageBlockParam::try_from(
                    &part.image_url,
                )?),
            ),
            chat::ChatCompletionRequestUserMessageContentPart::File(part) => Ok(
                anthropic::ContentBlockParam::Document(anthropic::DocumentBlockParam::try_from(
                    &part.file,
                )?),
            ),
            chat::ChatCompletionRequestUserMessageContentPart::InputAudio(_) => Err(
                TranslationError::InvalidPayload(
                    "Chat Completions input_audio user content cannot be translated to Anthropic Messages content"
                        .to_string(),
                ),
            ),
        }
    }
}

impl TryFrom<&chat::ImageUrl> for anthropic::ImageBlockParam {
    type Error = TranslationError;

    fn try_from(image: &chat::ImageUrl) -> TranslationResult<Self> {
        let source = if let Some((media_type, data)) = parse_base64_image_data_url(&image.url)? {
            anthropic::ImageBlockSource::Base64(anthropic::Base64ImageSource { data, media_type })
        } else {
            anthropic::ImageBlockSource::Url(anthropic::UrlImageSource {
                url: image.url.clone(),
            })
        };

        Ok(anthropic::ImageBlockParam {
            source,
            cache_control: None,
        })
    }
}

impl TryFrom<&str> for anthropic::ImageMediaType {
    type Error = TranslationError;

    fn try_from(value: &str) -> TranslationResult<Self> {
        match value {
            "image/jpeg" => Ok(Self::Jpeg),
            "image/png" => Ok(Self::Png),
            "image/gif" => Ok(Self::Gif),
            "image/webp" => Ok(Self::Webp),
            other => Err(TranslationError::InvalidPayload(format!(
                "Chat Completions image media type `{other}` cannot be translated to Anthropic Messages image content"
            ))),
        }
    }
}

fn parse_base64_image_data_url(
    url: &str,
) -> TranslationResult<Option<(anthropic::ImageMediaType, String)>> {
    let Some(rest) = url.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((media_type, data)) = rest.split_once(";base64,") else {
        return Err(TranslationError::InvalidPayload(
            "Chat Completions image data URLs must use ';base64,' encoding to translate to Anthropic Messages"
                .to_string(),
        ));
    };
    let media_type = anthropic::ImageMediaType::try_from(media_type)?;
    Ok(Some((media_type, data.to_string())))
}

impl TryFrom<&chat::ChatCompletionRequestAssistantMessage> for anthropic::MessageParamContent {
    type Error = TranslationError;

    fn try_from(message: &chat::ChatCompletionRequestAssistantMessage) -> TranslationResult<Self> {
        let mut blocks = Vec::new();
        if let Some(content) = &message.content {
            match content {
                chat::ChatCompletionRequestAssistantMessageContent::Text(text) => {
                    if !text.is_empty() {
                        blocks.push(anthropic::ContentBlockParam::Text(
                            anthropic::TextBlockParam {
                                text: text.clone(),
                                cache_control: None,
                                citations: None,
                            },
                        ));
                    }
                }
                chat::ChatCompletionRequestAssistantMessageContent::Array(parts) => {
                    for part in parts {
                        match part {
                            chat::ChatCompletionRequestAssistantMessageContentPart::Text(part)
                                if !part.text.is_empty() =>
                            {
                                blocks.push(anthropic::ContentBlockParam::Text(
                                    anthropic::TextBlockParam {
                                        text: part.text.clone(),
                                        cache_control: None,
                                        citations: None,
                                    },
                                ));
                            }
                            chat::ChatCompletionRequestAssistantMessageContentPart::Text(_) => {}
                            chat::ChatCompletionRequestAssistantMessageContentPart::Refusal(_) => {
                                return Err(TranslationError::InvalidPayload(
                                    "Chat Completions assistant refusal content cannot be translated to Anthropic Messages request content"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        for tool_call in message.tool_calls.iter().flatten() {
            match tool_call {
                chat::ChatCompletionMessageToolCalls::Function(tool_call) => {
                    let input = serde_json::from_str::<Value>(&tool_call.function.arguments)
                        .map_err(|error| {
                            TranslationError::InvalidPayload(format!(
                                "Chat Completions tool call `{}` arguments must be valid JSON to translate to Anthropic Messages: {error}",
                                tool_call.id
                            ))
                        })?;
                    blocks.push(anthropic::ContentBlockParam::ToolUse(
                        anthropic::ToolUseBlockParam {
                            id: tool_call.id.clone(),
                            input,
                            name: tool_call.function.name.clone(),
                            cache_control: None,
                            caller: None,
                        },
                    ));
                }
                chat::ChatCompletionMessageToolCalls::Custom(_) => {
                    return Err(TranslationError::InvalidPayload(
                        "Chat Completions custom tool calls cannot be translated to Anthropic Messages tool_use blocks"
                            .to_string(),
                    ));
                }
            }
        }

        match blocks.len() {
            0 => Err(TranslationError::InvalidPayload(
                "Chat Completions assistant message without content or tool calls cannot be translated to Anthropic Messages"
                    .to_string(),
            )),
            1 => match blocks.pop() {
                Some(anthropic::ContentBlockParam::Text(block)) => {
                    Ok(anthropic::MessageParamContent::Text(block.text))
                }
                Some(block) => Ok(anthropic::MessageParamContent::Blocks(vec![block])),
                None => Err(TranslationError::InvalidPayload(
                    "Chat Completions assistant message without content or tool calls cannot be translated to Anthropic Messages"
                        .to_string(),
                )),
            },
            _ => Ok(anthropic::MessageParamContent::Blocks(blocks)),
        }
    }
}

impl From<&chat::ChatCompletionRequestToolMessage> for anthropic::ToolResultBlockParam {
    fn from(message: &chat::ChatCompletionRequestToolMessage) -> Self {
        Self {
            tool_use_id: message.tool_call_id.clone(),
            content: Some(match &message.content {
                chat::ChatCompletionRequestToolMessageContent::Text(text) => {
                    anthropic::ToolResultContentParam::Text(text.clone())
                }
                chat::ChatCompletionRequestToolMessageContent::Array(parts) => {
                    anthropic::ToolResultContentParam::Blocks(
                        parts
                            .iter()
                            .map(|part| match part {
                                chat::ChatCompletionRequestToolMessageContentPart::Text(part) => {
                                    anthropic::ToolResultContentBlockParam::Text(
                                        anthropic::TextBlockParam {
                                            text: part.text.clone(),
                                            cache_control: None,
                                            citations: None,
                                        },
                                    )
                                }
                            })
                            .collect(),
                    )
                }
            }),
            // Chat tool messages have no standard error marker, so translate
            // them as successful Anthropic tool results.
            is_error: Some(false),
            cache_control: None,
        }
    }
}
