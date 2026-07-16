//! Request-side OpenAI Responses constructors and helpers.
//!
//! Used by `* -> openai_responses` request translators to build
//! protocol-native request types without repeating `None` field noise.

use crate::protocol::openai_responses as responses;

// ── Input content ─────────────────────────────────────────────────────────

pub(crate) fn input_text(text: impl Into<String>) -> responses::InputContent {
    responses::InputContent::InputText(responses::InputTextContent {
        text: text.into(),
        prompt_cache_breakpoint: None,
    })
}

pub(crate) fn input_image_url(url: impl Into<String>) -> responses::InputContent {
    responses::InputContent::InputImage(responses::InputImageContent {
        image_url: Some(url.into()).into(),
        detail: responses::ImageDetail::Auto,
        file_id: None.into(),
        prompt_cache_breakpoint: None,
    })
}

pub(crate) fn input_file_data(
    data: impl Into<String>,
    filename: Option<String>,
) -> responses::InputContent {
    responses::InputContent::InputFile(responses::InputFileContent {
        file_data: Some(data.into()),
        file_id: None.into(),
        file_url: None,
        filename,
        detail: None,
        prompt_cache_breakpoint: None,
    })
}

pub(crate) fn input_file_url(
    url: impl Into<String>,
    filename: Option<String>,
) -> responses::InputContent {
    responses::InputContent::InputFile(responses::InputFileContent {
        file_data: None,
        file_id: None.into(),
        file_url: Some(url.into()),
        filename,
        detail: None,
        prompt_cache_breakpoint: None,
    })
}

// ── Input items ───────────────────────────────────────────────────────────

pub(crate) fn easy_message(
    role: responses::Role,
    content: responses::EasyInputContent,
) -> responses::InputItem {
    responses::InputItem::EasyMessage(responses::EasyInputMessage {
        r#type: Some(responses::MessageType::Message),
        role,
        content,
        phase: None.into(),
    })
}

/// Build a request-side `FunctionToolCall` (status: None.into(),
/// because the tool has not been executed yet).
pub(crate) fn pending_function_tool_call(
    call_id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl Into<String>,
) -> responses::FunctionToolCall {
    responses::FunctionToolCall {
        call_id: call_id.into(),
        caller: None.into(),
        name: name.into(),
        arguments: arguments.into(),
        id: None,
        namespace: None,
        status: None,
    }
}

/// Build a `FunctionCallOutputItemParam` for a request-side tool result.
pub(crate) fn function_call_output_item(
    call_id: impl Into<String>,
    output: responses::FunctionCallOutput,
) -> responses::Item {
    responses::Item::FunctionCallOutput(responses::FunctionCallOutputItemParam {
        call_id: call_id.into(),
        output,
        caller: None.into(),
        id: None.into(),
        status: None.into(),
    })
}

// ── ID normalization ──────────────────────────────────────────────────────

/// Normalize an upstream id into a Responses-shaped id by ensuring it
/// starts with `resp_`.
pub(crate) fn response_id(id: &str) -> String {
    if id.starts_with("resp_") {
        id.to_string()
    } else {
        format!("resp_{id}")
    }
}
