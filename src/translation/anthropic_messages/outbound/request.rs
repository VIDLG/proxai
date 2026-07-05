//! Request-side Anthropic Messages constructors and helpers.
//!
//! Used by `* -> anthropic_messages` request translators to build
//! protocol-native request types without repeating `None` field noise.

use serde_json::{Value, json};
use url::Url;

use crate::protocol::anthropic::messages as anthropic;
use crate::translation::{TranslationError, TranslationResult};

// ── Request defaults ─────────────────────────────────────────────────────

// Anthropic Messages requires `max_tokens`, while OpenAI-compatible source
// protocols can omit token-limit fields. This is a proxai compatibility
// fallback for those clients; it is not an upstream protocol default.
pub(crate) const COMPATIBILITY_MAX_TOKENS_FALLBACK: u32 = 4096;

// ── JSON scalar helpers ──────────────────────────────────────────────────

pub(crate) fn json_number_from_f32(value: f32) -> Option<serde_json::Number> {
    serde_json::Number::from_f64(f64::from(value))
}

// ── Text blocks ───────────────────────────────────────────────────────────

pub(crate) fn text_block_param(text: impl Into<String>) -> anthropic::TextBlockParam {
    anthropic::TextBlockParam {
        text: text.into(),
        cache_control: None,
        citations: None,
    }
}

pub(crate) fn typed_text_block(text: impl Into<String>) -> anthropic::TypedTextBlockParam {
    anthropic::TypedTextBlockParam {
        type_: anthropic::TextBlockType::Text,
        text: text.into(),
        cache_control: None,
        citations: None,
    }
}

// ── Messages ─────────────────────────────────────────────────────────────

pub(crate) fn message(
    role: anthropic::MessageParamRole,
    content: anthropic::MessageParamContent,
) -> anthropic::MessageParam {
    anthropic::MessageParam { role, content }
}

pub(crate) fn user_message(content: anthropic::MessageParamContent) -> anthropic::MessageParam {
    message(anthropic::MessageParamRole::User, content)
}

pub(crate) fn assistant_message(
    content: anthropic::MessageParamContent,
) -> anthropic::MessageParam {
    message(anthropic::MessageParamRole::Assistant, content)
}

pub(crate) fn content_block_message(
    role: anthropic::MessageParamRole,
    block: anthropic::ContentBlockParam,
) -> anthropic::MessageParam {
    message(role, anthropic::MessageParamContent::Blocks(vec![block]))
}

pub(crate) fn merge_adjacent_tool_messages(
    messages: Vec<anthropic::MessageParam>,
) -> Vec<anthropic::MessageParam> {
    let mut merged: Vec<anthropic::MessageParam> = Vec::new();
    for message in messages {
        let should_merge = merged.last().is_some_and(|last| {
            last.role == message.role && is_tool_message(last) && is_tool_message(&message)
        });
        if should_merge {
            if let (
                Some(anthropic::MessageParam {
                    content: anthropic::MessageParamContent::Blocks(target_blocks),
                    ..
                }),
                anthropic::MessageParamContent::Blocks(source_blocks),
            ) = (merged.last_mut(), message.content)
            {
                target_blocks.extend(source_blocks);
            }
        } else {
            merged.push(message);
        }
    }
    merged
}

fn is_tool_message(message: &anthropic::MessageParam) -> bool {
    match (message.role, &message.content) {
        (
            anthropic::MessageParamRole::Assistant,
            anthropic::MessageParamContent::Blocks(blocks),
        ) => {
            !blocks.is_empty()
                && blocks
                    .iter()
                    .all(|block| matches!(block, anthropic::ContentBlockParam::ToolUse(_)))
        }
        (anthropic::MessageParamRole::User, anthropic::MessageParamContent::Blocks(blocks)) => {
            !blocks.is_empty()
                && blocks
                    .iter()
                    .all(|block| matches!(block, anthropic::ContentBlockParam::ToolResult(_)))
        }
        _ => false,
    }
}

// ── Output config ────────────────────────────────────────────────────────

pub(crate) fn output_config(effort: anthropic::OutputEffort) -> anthropic::OutputConfig {
    anthropic::OutputConfig {
        effort: Some(effort),
        format: None,
    }
}

// ── System prompt ────────────────────────────────────────────────────────

/// Build an Anthropic `SystemPrompt` from collected text parts.
///
/// - 0 non-empty parts → `None`
/// - 1 part → `SystemPrompt::Text`
/// - N parts → `SystemPrompt::Blocks` with one `TypedTextBlockParam` per part
pub(crate) fn system_prompt_from_text_parts(parts: Vec<String>) -> Option<anthropic::SystemPrompt> {
    let parts: Vec<String> = parts
        .into_iter()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect();

    match parts.len() {
        0 => None,
        1 => parts.into_iter().next().map(anthropic::SystemPrompt::Text),
        _ => Some(anthropic::SystemPrompt::Blocks(
            parts.into_iter().map(typed_text_block).collect(),
        )),
    }
}

// ── Image ────────────────────────────────────────────────────────────────

/// Build an Anthropic `ImageBlockParam` from a Chat/Responses-style image URL
/// string. Supports `data:<media>;base64,...` image data URLs and `http(s)` URLs.
pub(crate) fn image_block_from_url(url: &str) -> TranslationResult<anthropic::ImageBlockParam> {
    let source = if let Some((media_type, data)) = parse_base64_image_data_url(url)? {
        anthropic::ImageBlockSource::Base64(anthropic::Base64ImageSource { data, media_type })
    } else if is_http_url(url) {
        anthropic::ImageBlockSource::Url(anthropic::UrlImageSource {
            url: url.to_string(),
        })
    } else {
        return Err(TranslationError::InvalidPayload(
            "Image URL values must be an http(s) URL or data:image/<type>;base64,<data> to translate to Anthropic Messages image content"
                .to_string(),
        ));
    };

    Ok(anthropic::ImageBlockParam {
        source,
        cache_control: None,
    })
}

fn parse_base64_image_data_url(
    url: &str,
) -> TranslationResult<Option<(anthropic::ImageMediaType, String)>> {
    let Some(rest) = url.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((media_type, data)) = rest.split_once(";base64,") else {
        return Err(TranslationError::InvalidPayload(
            "Image data URLs must use ';base64,' encoding to translate to Anthropic Messages image content".to_string(),
        ));
    };
    let media_type = image_media_type(media_type)?;
    Ok(Some((media_type, data.to_string())))
}

fn image_media_type(value: &str) -> TranslationResult<anthropic::ImageMediaType> {
    match value {
        "image/jpeg" => Ok(anthropic::ImageMediaType::Jpeg),
        "image/png" => Ok(anthropic::ImageMediaType::Png),
        "image/gif" => Ok(anthropic::ImageMediaType::Gif),
        "image/webp" => Ok(anthropic::ImageMediaType::Webp),
        other => Err(TranslationError::InvalidPayload(format!(
            "Image media type `{other}` cannot be translated to Anthropic Messages image content"
        ))),
    }
}

fn is_http_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
}

// ── Documents ─────────────────────────────────────────────────────────────

pub(crate) fn document_source_from_url(
    url: &str,
) -> TranslationResult<anthropic::DocumentBlockParamSource> {
    if !is_pdf_url(url) {
        return Err(TranslationError::InvalidPayload(
            "Document URL values must be an http(s) PDF URL to translate to Anthropic Messages document content"
                .to_string(),
        ));
    }

    Ok(anthropic::DocumentBlockParamSource::Url(
        anthropic::UrlPdfSource {
            url: url.to_string(),
        },
    ))
}

pub(crate) fn document_source_from_file_data(
    data: &str,
) -> TranslationResult<anthropic::DocumentBlockParamSource> {
    let Some(rest) = data.strip_prefix("data:") else {
        return Ok(plain_text_document_source(data));
    };

    if let Some(source) = base64_pdf_document_source(rest) {
        return Ok(source);
    }

    if let Some(text) = rest.strip_prefix("text/plain,") {
        return Ok(plain_text_document_source(text));
    }

    if rest.starts_with("text/plain;base64,") {
        return Err(TranslationError::InvalidPayload(
            "text/plain;base64 file data cannot be translated to Anthropic Messages without decoding; provide plain text file_data instead"
                .to_string(),
        ));
    }

    Err(TranslationError::InvalidPayload(
        "file_data must be raw text, data:text/plain,<text>, or data:application/pdf;base64,<data> to translate to Anthropic Messages document content"
            .to_string(),
    ))
}

pub(crate) fn pdf_document_source_from_file_data_or_url(
    data: &str,
) -> TranslationResult<anthropic::DocumentBlockParamSource> {
    if let Some(rest) = data.strip_prefix("data:")
        && let Some(source) = base64_pdf_document_source(rest)
    {
        return Ok(source);
    }

    if is_http_url(data) {
        return document_source_from_url(data);
    }

    Err(TranslationError::InvalidPayload(
        "file_data can only be translated to Anthropic Messages document content when it is a PDF data URL or PDF URL"
            .to_string(),
    ))
}

fn base64_pdf_document_source(rest: &str) -> Option<anthropic::DocumentBlockParamSource> {
    rest.strip_prefix("application/pdf;base64,").map(|data| {
        anthropic::DocumentBlockParamSource::Base64(anthropic::Base64PdfSource {
            media_type: anthropic::PdfMediaType::ApplicationPdf,
            data: data.to_string(),
        })
    })
}

fn plain_text_document_source(data: &str) -> anthropic::DocumentBlockParamSource {
    anthropic::DocumentBlockParamSource::PlainText(anthropic::PlainTextSource {
        media_type: anthropic::PlainTextMediaType::TextPlain,
        data: data.to_string(),
    })
}

fn is_pdf_url(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.path().to_ascii_lowercase().ends_with(".pdf")
    })
}

// ── Tool use / tool result ───────────────────────────────────────────────

pub(crate) fn tool_use_block_param(
    id: impl Into<String>,
    name: impl Into<String>,
    input: Value,
) -> anthropic::ToolUseBlockParam {
    anthropic::ToolUseBlockParam {
        id: id.into(),
        input,
        name: name.into(),
        cache_control: None,
        caller: None,
    }
}

// ── Tools ────────────────────────────────────────────────────────────────

/// Build an Anthropic `InputSchema` from a Chat/Responses-style JSON schema
/// object (`{ type, properties, required, ... }`).
pub(crate) fn input_schema(parameters: Option<&Value>) -> anthropic::InputSchema {
    let Some(Value::Object(parameters)) = parameters else {
        return anthropic::InputSchema::default();
    };

    let mut extra = parameters.clone();
    let type_ = match extra.remove("type") {
        Some(Value::String(value)) => value,
        _ => "object".to_string(),
    };
    let properties = extra.remove("properties").or_else(|| Some(json!({})));
    let required = extra
        .remove("required")
        .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
        .or_else(|| Some(Vec::new()));

    anthropic::InputSchema {
        type_,
        properties,
        required,
        extra: Value::Object(extra),
    }
}

/// Build an Anthropic custom `ToolUnion` from a function-style tool definition.
///
/// When `parameters` is `None`, an empty schema (`InputSchema::default()`)
/// is used. Pass `strict: None` for tools that have no strictness semantics
/// (e.g. Responses custom tools).
pub(crate) fn custom_tool(
    name: impl Into<String>,
    description: Option<String>,
    parameters: Option<&Value>,
    strict: Option<bool>,
    defer_loading: Option<bool>,
) -> anthropic::ToolUnion {
    anthropic::ToolUnion::Custom(anthropic::Tool {
        input_schema: input_schema(parameters),
        name: name.into(),
        allowed_callers: None,
        cache_control: None,
        defer_loading,
        description,
        eager_input_streaming: None,
        input_examples: None,
        strict,
        type_: None,
    })
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
