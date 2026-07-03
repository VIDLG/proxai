//! Request-side Anthropic Messages constructors and helpers.
//!
//! Used by `* -> anthropic_messages` request translators to build
//! protocol-native request types without repeating `None` field noise.

use serde_json::{Value, json};

use crate::protocol::anthropic::messages as anthropic;
use crate::translation::{TranslationError, TranslationResult};

// ── Text blocks ───────────────────────────────────────────────────────────

pub(crate) fn text_block(text: impl Into<String>) -> anthropic::TextBlockParam {
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
/// string. Supports both `data:<media>;base64,...` data URLs and plain URLs.
pub(crate) fn image_block_from_url(url: &str) -> TranslationResult<anthropic::ImageBlockParam> {
    let source = if let Some((media_type, data)) = parse_base64_image_data_url(url)? {
        anthropic::ImageBlockSource::Base64(anthropic::Base64ImageSource { data, media_type })
    } else {
        anthropic::ImageBlockSource::Url(anthropic::UrlImageSource {
            url: url.to_string(),
        })
    };

    Ok(anthropic::ImageBlockParam {
        source,
        cache_control: None,
    })
}

pub(crate) fn url_image_block(url: impl Into<String>) -> anthropic::ImageBlockParam {
    anthropic::ImageBlockParam {
        source: anthropic::ImageBlockSource::Url(anthropic::UrlImageSource { url: url.into() }),
        cache_control: None,
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

// ── Tool use / tool result ───────────────────────────────────────────────

pub(crate) fn tool_use_block(
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

pub(crate) fn tool_result_text(
    tool_use_id: impl Into<String>,
    content: impl Into<String>,
) -> anthropic::ToolResultBlockParam {
    anthropic::ToolResultBlockParam {
        tool_use_id: tool_use_id.into(),
        content: Some(anthropic::ToolResultContentParam::Text(content.into())),
        is_error: Some(false),
        cache_control: None,
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
