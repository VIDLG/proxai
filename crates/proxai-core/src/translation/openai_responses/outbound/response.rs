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
        logprobs: Vec::new(),
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
        phase: None.into(),
    }
}

pub(crate) fn in_progress_message_item(id: impl Into<String>) -> responses::OutputItem {
    responses::OutputItem::Message(responses::OutputMessage {
        id: id.into(),
        role: responses::AssistantRole::Assistant,
        status: responses::OutputStatus::InProgress,
        content: Vec::new(),
        phase: None.into(),
    })
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

pub(crate) fn refusal_message_item(
    id: impl Into<String>,
    refusal: impl Into<String>,
) -> responses::OutputItem {
    responses::OutputItem::Message(assistant_message(
        id,
        vec![responses::OutputMessageContent::Refusal(
            responses::RefusalContent {
                refusal: refusal.into(),
            },
        )],
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
        caller: None.into(),
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

pub(crate) fn in_progress_function_call_item(
    item_id: impl Into<String>,
    name: impl Into<String>,
) -> responses::OutputItem {
    let item_id = item_id.into();
    responses::OutputItem::FunctionCall(responses::FunctionToolCall {
        id: Some(item_id.clone()),
        call_id: item_id,
        caller: None.into(),
        name: name.into(),
        arguments: String::new(),
        status: Some(responses::OutputStatus::InProgress),
        namespace: None,
    })
}

pub(crate) fn completed_function_call_item_with_id(
    item_id: impl Into<String>,
    name: impl Into<String>,
    arguments: impl Into<String>,
) -> responses::OutputItem {
    let item_id = item_id.into();
    responses::OutputItem::FunctionCall(responses::FunctionToolCall {
        id: Some(item_id.clone()),
        call_id: item_id,
        caller: None.into(),
        name: name.into(),
        arguments: arguments.into(),
        status: Some(responses::OutputStatus::Completed),
        namespace: None,
    })
}

pub(crate) fn reasoning_item(
    id: impl Into<String>,
    text: impl Into<String>,
) -> responses::OutputItem {
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        content: Some(vec![responses::ReasoningItemContent::ReasoningText(
            responses::ReasoningTextContent { text: text.into() },
        )]),
        encrypted_content: None.into(),
        status: Some(responses::OutputStatus::Completed),
    })
}

pub(crate) fn in_progress_reasoning_item(id: impl Into<String>) -> responses::OutputItem {
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        content: Some(Vec::new()),
        encrypted_content: None.into(),
        status: Some(responses::OutputStatus::InProgress),
    })
}

pub(crate) fn in_progress_redacted_reasoning_item(id: impl Into<String>) -> responses::OutputItem {
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        content: None,
        encrypted_content: None.into(),
        status: Some(responses::OutputStatus::InProgress),
    })
}

pub(crate) fn redacted_reasoning_item(
    id: impl Into<String>,
    data: impl Into<String>,
) -> responses::OutputItem {
    let data: String = data.into();
    responses::OutputItem::Reasoning(responses::ReasoningItem {
        id: id.into(),
        summary: Vec::new(),
        content: None,
        encrypted_content: Some(data).into(),
        status: Some(responses::OutputStatus::Completed),
    })
}
