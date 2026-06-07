use serde_json::{Value, json};

use crate::protocol::anthropic::messages as anthropic;
use crate::protocol::openai::chat_completions as chat;
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn anthropic_message(
    role: anthropic::Role,
    content: anthropic::MessageParamContent,
) -> anthropic::MessageParam {
    anthropic::MessageParam { role, content }
}

pub(super) fn join_text_parts(parts: Vec<String>) -> Option<String> {
    let text = parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    (!text.is_empty()).then_some(text)
}

pub(super) fn collect_developer_content(
    content: &chat::ChatCompletionRequestDeveloperMessageContent,
    out: &mut Vec<String>,
) {
    match content {
        chat::ChatCompletionRequestDeveloperMessageContent::Text(text) => out.push(text.clone()),
        chat::ChatCompletionRequestDeveloperMessageContent::Array(parts) => {
            out.extend(parts.iter().map(|part| match part {
                chat::ChatCompletionRequestDeveloperMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            }));
        }
    }
}

pub(super) fn collect_system_content(
    content: &chat::ChatCompletionRequestSystemMessageContent,
    out: &mut Vec<String>,
) {
    match content {
        chat::ChatCompletionRequestSystemMessageContent::Text(text) => out.push(text.clone()),
        chat::ChatCompletionRequestSystemMessageContent::Array(parts) => {
            out.extend(parts.iter().map(|part| match part {
                chat::ChatCompletionRequestSystemMessageContentPart::Text(part) => {
                    part.text.clone()
                }
            }));
        }
    }
}

fn text_block(text: String) -> anthropic::ContentBlockParam {
    anthropic::ContentBlockParam::Text(anthropic::TextBlockParam {
        text,
        cache_control: None,
        citations: None,
    })
}

pub(super) fn user_content(
    content: &chat::ChatCompletionRequestUserMessageContent,
) -> TranslationResult<anthropic::MessageParamContent> {
    match content {
        chat::ChatCompletionRequestUserMessageContent::Text(text) => {
            Ok(anthropic::MessageParamContent::Text(text.clone()))
        }
        chat::ChatCompletionRequestUserMessageContent::Array(parts) => {
            let blocks = parts
                .iter()
                .map(anthropic_user_content_block)
                .collect::<TranslationResult<Vec<_>>>()?;
            if blocks.is_empty() {
                Ok(anthropic::MessageParamContent::Text(String::new()))
            } else {
                Ok(anthropic::MessageParamContent::Blocks(blocks))
            }
        }
    }
}

fn anthropic_user_content_block(
    part: &chat::ChatCompletionRequestUserMessageContentPart,
) -> TranslationResult<anthropic::ContentBlockParam> {
    match part {
        chat::ChatCompletionRequestUserMessageContentPart::Text(part) => {
            Ok(text_block(part.text.clone()))
        }
        chat::ChatCompletionRequestUserMessageContentPart::ImageUrl(part) => {
            Ok(anthropic::ContentBlockParam::Image(anthropic_image_block(
                &part.image_url,
            )?))
        }
        chat::ChatCompletionRequestUserMessageContentPart::File(part) => {
            Ok(anthropic::ContentBlockParam::Document(anthropic_file_block(
                &part.file,
            )?))
        }
        chat::ChatCompletionRequestUserMessageContentPart::InputAudio(_) => Err(
            TranslationError::InvalidPayload(
                "Chat Completions input_audio user content cannot be translated to Anthropic Messages content"
                    .to_string(),
            ),
        ),
    }
}

fn anthropic_image_block(image: &chat::ImageUrl) -> TranslationResult<anthropic::ImageBlockParam> {
    let source = if let Some((media_type, data)) = parse_image_data_url(&image.url)? {
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

fn anthropic_file_block(
    file: &chat::FileObject,
) -> TranslationResult<anthropic::DocumentBlockParam> {
    let Some(file_data) = file.file_data.as_deref() else {
        return Err(TranslationError::InvalidPayload(
            "Chat Completions file user content with only file_id cannot be translated to Anthropic Messages document content"
                .to_string(),
        ));
    };

    let data = if let Some(data) = file_data.strip_prefix("data:application/pdf;base64,") {
        data.to_string()
    } else if file
        .filename
        .as_deref()
        .is_some_and(|filename| filename.to_ascii_lowercase().ends_with(".pdf"))
    {
        file_data.to_string()
    } else {
        return Err(TranslationError::InvalidPayload(
            "Chat Completions file user content can only be translated to Anthropic Messages when it contains base64 PDF data"
                .to_string(),
        ));
    };

    Ok(anthropic::DocumentBlockParam {
        source: anthropic::DocumentBlockParamSource::Base64(anthropic::Base64PdfSource {
            data,
            media_type: anthropic::PdfMediaType::ApplicationPdf,
        }),
        cache_control: None,
        citations: None,
        context: None,
        title: file.filename.clone(),
    })
}

fn parse_image_data_url(
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
    let media_type = match media_type {
        "image/jpeg" => anthropic::ImageMediaType::Jpeg,
        "image/png" => anthropic::ImageMediaType::Png,
        "image/gif" => anthropic::ImageMediaType::Gif,
        "image/webp" => anthropic::ImageMediaType::Webp,
        other => {
            return Err(TranslationError::InvalidPayload(format!(
                "Chat Completions image media type `{other}` cannot be translated to Anthropic Messages image content"
            )));
        }
    };
    Ok(Some((media_type, data.to_string())))
}

pub(super) fn assistant_content(
    message: &chat::ChatCompletionRequestAssistantMessage,
) -> anthropic::MessageParamContent {
    let mut blocks = Vec::new();
    if let Some(content) = &message.content {
        match content {
            chat::ChatCompletionRequestAssistantMessageContent::Text(text) => {
                if !text.is_empty() {
                    blocks.push(text_block(text.clone()));
                }
            }
            chat::ChatCompletionRequestAssistantMessageContent::Array(parts) => {
                for part in parts {
                    if let chat::ChatCompletionRequestAssistantMessageContentPart::Text(part) = part
                    {
                        blocks.push(text_block(part.text.clone()));
                    }
                }
            }
        }
    }

    for tool_call in message.tool_calls.iter().flatten() {
        match tool_call {
            chat::ChatCompletionMessageToolCalls::Function(tool_call) => {
                let input = serde_json::from_str::<Value>(&tool_call.function.arguments)
                    .unwrap_or_else(|_| json!({}));
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
            chat::ChatCompletionMessageToolCalls::Custom(tool_call) => {
                blocks.push(anthropic::ContentBlockParam::ToolUse(
                    anthropic::ToolUseBlockParam {
                        id: tool_call.id.clone(),
                        input: Value::String(tool_call.custom_tool.input.clone()),
                        name: tool_call.custom_tool.name.clone(),
                        cache_control: None,
                        caller: None,
                    },
                ));
            }
        }
    }

    if blocks.is_empty() {
        anthropic::MessageParamContent::Text(String::new())
    } else {
        anthropic::MessageParamContent::Blocks(blocks)
    }
}

pub(super) fn tool_content_as_text(
    content: &chat::ChatCompletionRequestToolMessageContent,
) -> String {
    match content {
        chat::ChatCompletionRequestToolMessageContent::Text(text) => text.clone(),
        chat::ChatCompletionRequestToolMessageContent::Array(parts) => parts
            .iter()
            .map(|part| match part {
                chat::ChatCompletionRequestToolMessageContentPart::Text(part) => part.text.clone(),
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
    }
}
