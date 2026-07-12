use crate::protocol::openai::chat_completions::{
    ChatChoice, ChatCompletionMessageToolCalls, CreateChatCompletionResponse,
    CreateChatCompletionResponseObject, FinishReason,
};
use crate::protocol::openai::responses::{
    OutputItem, OutputMessage, OutputMessageContent, Response,
};
use crate::translation::openai_chat_completions::outbound::assistant_response_message;
use crate::translation::openai_responses::stop::{ResponsesStopKind, infer_response_stop_kind};
use crate::translation::{TranslationError, TranslationResult};

use super::types::chat_id;

pub(super) fn translate_response(
    response: &Response,
) -> TranslationResult<(CreateChatCompletionResponse, Option<String>)> {
    let mut content_parts: Vec<String> = Vec::new();
    let mut reasoning_parts: Vec<String> = Vec::new();
    let mut refusal_parts: Vec<String> = Vec::new();
    let mut tool_calls = Vec::new();

    for output in &response.output {
        match output {
            OutputItem::Message(message) => {
                collect_message_content(message, &mut content_parts, &mut refusal_parts);
            }
            OutputItem::FunctionCall(call) => {
                tool_calls.push(ChatCompletionMessageToolCalls::from(call));
            }
            OutputItem::CustomToolCall(call) => {
                tool_calls.push(ChatCompletionMessageToolCalls::from(call));
            }
            OutputItem::Reasoning(reasoning) => {
                reasoning_parts.extend(reasoning.summary.iter().map(|part| match part {
                    crate::protocol::openai::responses::SummaryPart::SummaryText(text) => {
                        text.text.clone()
                    }
                }));
                if let Some(content) = reasoning.content.as_ref() {
                    reasoning_parts.extend(content.iter().map(|part| match part {
                        crate::protocol::openai::responses::ReasoningItemContent::ReasoningText(
                            text,
                        ) => text.text.clone(),
                    }));
                }
                if reasoning.encrypted_content.is_non_null()
                    && reasoning.summary.is_empty()
                    && reasoning.content.as_ref().is_none_or(Vec::is_empty)
                {
                    tracing::trace!(
                        reason = "Chat reasoning_content cannot represent encrypted reasoning",
                        "skipping encrypted Responses reasoning item during Chat Completions translation"
                    );
                }
            }
            skipped => {
                // Responses output items without a Chat Completions
                // representation (reasoning, hosted tool calls, MCP, search,
                // compaction, etc.) are skipped with a trace log to keep
                // silent drops observable.
                tracing::trace!(
                    discriminant = ?std::mem::discriminant(skipped),
                    "skipping Responses output item with no Chat-representable field"
                );
            }
        }
    }

    let content = content_parts.join("");
    let reasoning_content = reasoning_parts.concat();
    let refusal = refusal_parts.join("");
    if !content.is_empty() && !refusal.is_empty() {
        return Err(TranslationError::InvalidPayload(
                "OpenAI Responses output contains both text and refusal content; Chat Completions response cannot represent mixed refusal semantics"
                    .to_string(),
            ));
    }
    if content.is_empty()
        && reasoning_content.is_empty()
        && refusal.is_empty()
        && tool_calls.is_empty()
    {
        return Err(TranslationError::InvalidPayload(
            "OpenAI Responses output has no Chat-representable text, reasoning, or tool calls"
                .to_string(),
        ));
    }

    Ok((CreateChatCompletionResponse {
            // Keep the upstream id embedded while presenting an OpenAI-shaped id.
            id: chat_id(&response.id),
            choices: vec![ChatChoice {
                index: 0,
                message: assistant_response_message(
                    (!content.is_empty()).then_some(content),
                    (!refusal.is_empty()).then_some(refusal),
                    (!tool_calls.is_empty()).then_some(tool_calls),
                    None,
                ),
                finish_reason: chat_finish_reason(response).ok_or_else(|| {
                    TranslationError::InvalidPayload(
                        "OpenAI Responses response has no terminal state required for Chat Completions finish_reason"
                            .to_string(),
                    )
                })?,
                logprobs: None.into(),
            }],
            // Responses responses carry a `created_at` Unix timestamp.
            created: response.created_at as u32,
            model: response.model.clone(),
            // Responses has no Chat-style service tier field on the response body.
            service_tier: None.into(),
            system_fingerprint: None,
            object: CreateChatCompletionResponseObject::ChatCompletion,
            usage: response.usage.as_ref().map(Into::into),
        }, (!reasoning_content.is_empty()).then_some(reasoning_content)))
}

impl TryFrom<&Response> for CreateChatCompletionResponse {
    type Error = TranslationError;

    fn try_from(response: &Response) -> TranslationResult<Self> {
        translate_response(response).map(|(response, _)| response)
    }
}

fn chat_finish_reason(response: &Response) -> Option<FinishReason> {
    infer_response_stop_kind(response).map(|kind| match kind {
        ResponsesStopKind::EndTurn | ResponsesStopKind::Refusal => FinishReason::Stop,
        ResponsesStopKind::MaxTokens => FinishReason::Length,
        ResponsesStopKind::ToolUse => FinishReason::ToolCalls,
    })
}

fn collect_message_content(
    message: &OutputMessage,
    content_parts: &mut Vec<String>,
    refusal_parts: &mut Vec<String>,
) {
    for content in &message.content {
        match content {
            OutputMessageContent::OutputText(text) => content_parts.push(text.text.clone()),
            OutputMessageContent::Refusal(refusal) => refusal_parts.push(refusal.refusal.clone()),
        }
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
