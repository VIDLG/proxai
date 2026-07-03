//! Response-side OpenAI Responses constructors.
//!
//! Used by `* -> openai_responses` response and streaming translators to
//! build protocol-native response types without repeating `None` field noise.

use crate::protocol::openai_responses as responses;

// ── Output content ────────────────────────────────────────────────────────

pub(crate) fn output_text(
    text: impl Into<String>,
    annotations: Vec<responses::Annotation>,
) -> responses::OutputTextContent {
    responses::OutputTextContent {
        text: text.into(),
        annotations,
        logprobs: None,
    }
}

// ── Output message ────────────────────────────────────────────────────────

pub(crate) fn assistant_message(
    id: impl Into<String>,
    content: Vec<responses::OutputMessageContent>,
) -> responses::OutputMessage {
    responses::OutputMessage {
        id: id.into(),
        role: responses::AssistantRole::Assistant,
        status: responses::OutputStatus::Completed,
        content,
        phase: None,
    }
}

pub(crate) fn text_message_item(
    id: impl Into<String>,
    text: impl Into<String>,
    annotations: Vec<responses::Annotation>,
) -> responses::OutputItem {
    responses::OutputItem::Message(assistant_message(
        id,
        vec![responses::OutputMessageContent::OutputText(output_text(
            text,
            annotations,
        ))],
    ))
}

/// Build a response-side `FunctionToolCall` (status: Completed).
pub(crate) fn completed_function_tool_call(
    call_id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl Into<String>,
) -> responses::FunctionToolCall {
    responses::FunctionToolCall {
        call_id: call_id.into(),
        name: name.into(),
        arguments: arguments.into(),
        id: None,
        namespace: None,
        status: Some(responses::OutputStatus::Completed),
    }
}

pub(crate) fn function_call_item(
    call_id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl Into<String>,
) -> responses::OutputItem {
    responses::OutputItem::FunctionCall(completed_function_tool_call(call_id, name, arguments))
}

pub(crate) fn reasoning_item(
    id: impl Into<String>,
    text: impl Into<String>,
) -> responses::OutputItem {
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: Some(id.into()),
        summary: Vec::new(),
        content: Some(vec![responses::ReasoningItemContent::ReasoningText(
            responses::ReasoningTextContent { text: text.into() },
        )]),
        encrypted_content: None,
        status: Some(responses::OutputStatus::Completed),
    })
}

pub(crate) fn redacted_reasoning_item(
    id: impl Into<String>,
    data: impl Into<String>,
) -> responses::OutputItem {
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: Some(id.into()),
        summary: Vec::new(),
        content: None,
        encrypted_content: Some(data.into()),
        status: Some(responses::OutputStatus::Completed),
    })
}
