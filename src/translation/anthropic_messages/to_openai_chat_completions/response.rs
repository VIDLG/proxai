//! Non-streaming response conversion for `anthropic_messages -> openai_chat_completions`.

use super::citations::text_block_annotations;
use super::types::chat_finish_reason_from_anthropic_stop_reason;
use crate::protocol::anthropic::messages::{ContentBlock, Message, StopReason};
use crate::protocol::openai::chat_completions::{
    ChatChoice, CreateChatCompletionResponse, CreateChatCompletionResponseObject,
};
use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::openai_chat_completions::outbound::assistant_response_message;
use crate::translation::{TranslationError, TranslationResult};

pub(super) fn translate_response(
    message: &Message,
) -> TranslationResult<(CreateChatCompletionResponse, Option<String>)> {
    let mut text_parts: Vec<String> = Vec::new();
    let mut reasoning_parts: Vec<String> = Vec::new();
    let mut continuation = ContinuationEnvelope::default();
    let mut tool_calls = Vec::new();
    let mut annotations = Vec::new();

    for block in &message.content {
        match block {
            ContentBlock::Text(block) => {
                let base_offset = text_parts.iter().map(|text| text.chars().count()).sum();
                annotations.extend(text_block_annotations(block, base_offset));
                text_parts.push(block.text.clone());
            }
            ContentBlock::ToolUse(block) => tool_calls.push(block.try_into()?),
            ContentBlock::Thinking(block) => {
                reasoning_parts.push(block.thinking.clone());
                continuation.push(Continuation::Thinking {
                    thinking: block.thinking.clone(),
                    signature: block.signature.clone(),
                });
            }
            ContentBlock::RedactedThinking(block) => {
                continuation.push(Continuation::RedactedThinking {
                    data: block.data.clone(),
                });
            }
            // Server-tool artifacts have no safe Chat message field.
            skipped => {
                tracing::trace!(
                    discriminant = ?std::mem::discriminant(skipped),
                    "skipping Anthropic response block with no Chat-representable field"
                );
            }
        }
    }

    // Chat Completions cannot preserve Anthropic block interleaving in a
    // non-streaming assistant message, so text blocks are flattened.
    let content = text_parts.join("");
    let reasoning_content = if continuation.is_empty() {
        reasoning_parts.concat()
    } else {
        continuation.append_to_chat_reasoning_content(reasoning_parts.concat())?
    };
    let refusal = message_refusal(message, &content);
    let content = if refusal.is_some() {
        String::new()
    } else {
        content
    };

    if content.is_empty()
        && reasoning_content.is_empty()
        && refusal.is_none()
        && tool_calls.is_empty()
    {
        return Err(TranslationError::InvalidPayload(
                "Anthropic message response has no Chat-representable content, thinking, refusal, or tool_use blocks"
                    .to_string(),
            ));
    }

    Ok((CreateChatCompletionResponse {
            // Anthropic message ids are not Chat Completion ids; keep the
            // upstream id embedded while presenting an OpenAI-compatible shape.
            id: format!("chatcmpl_{}", message.id),
            choices: vec![ChatChoice {
                index: 0,
                message: assistant_response_message(
                    (!content.is_empty()).then_some(content),
                    refusal,
                    (!tool_calls.is_empty()).then_some(tool_calls),
                    // Only Anthropic web-search citations can be represented as
                    // OpenAI Chat URL annotations. Other citation location types
                    // lack URL annotation equivalents.
                    (!annotations.is_empty()).then_some(annotations),
                ),
                finish_reason: message
                    .stop_reason
                    .as_non_null()
                    .copied()
                    .map(chat_finish_reason_from_anthropic_stop_reason)
                    .ok_or_else(|| {
                        TranslationError::InvalidPayload(
                            "Anthropic message response is missing stop_reason required by Chat Completions"
                                .to_string(),
                        )
                    })?,
                // Anthropic does not expose Chat-style token logprobs on message responses.
                logprobs: None.into(),
            }],
            // Anthropic message responses do not carry a Unix creation timestamp.
            created: 0,
            model: message.model.clone(),
            // Anthropic response `usage.service_tier` is not the same shape as OpenAI
            // Chat's response-level service tier, so avoid inventing a value.
            service_tier: None.into(),
            system_fingerprint: None,
            object: CreateChatCompletionResponseObject::ChatCompletion,
            usage: Some((&message.usage).into()),
        }, (!reasoning_content.is_empty()).then_some(reasoning_content)))
}

fn message_refusal(message: &Message, content: &str) -> Option<String> {
    if !matches!(message.stop_reason.as_non_null(), Some(StopReason::Refusal)) {
        return None;
    }

    // OpenAI Chat `refusal` is the refusal message generated by the model.
    // Anthropic text content is the closest user-visible refusal message;
    // `stop_details.explanation` explains why the model refused, so use it
    // only when Anthropic did not return visible refusal text.
    (!content.is_empty())
        .then(|| content.to_string())
        .or_else(|| {
            message
                .stop_details
                .as_non_null()
                .and_then(|details| details.explanation.as_non_null().cloned())
        })
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
