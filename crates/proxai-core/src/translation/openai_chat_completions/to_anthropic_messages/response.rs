//! Non-streaming response conversion for `openai_chat_completions -> anthropic_messages`.

use crate::protocol::anthropic::messages::{
    ContentBlock, Message, MessageRole as AnthropicMessageRole, MessageType, RedactedThinkingBlock,
    RefusalStopDetails, RefusalStopDetailsType, StopReason, ThinkingBlock, ToolUseBlock,
};
use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCalls, CreateChatCompletionResponse, FinishReason,
};
use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::anthropic_messages::outbound::{text_block, tool_use_block};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

use super::super::response::single_assistant_choice;
use super::types::anthropic_stop_reason_from_chat_finish_reason;

pub(super) fn translate_response(
    chat: &CreateChatCompletionResponse,
    reasoning_content: Option<&str>,
    scope: &TranslationScope,
) -> TranslationResult<Message> {
    let choice = single_assistant_choice(&chat.choices)?;
    if choice.logprobs.is_non_null() {
        return Err(TranslationError::InvalidPayload(
            "Chat completion response choice logprobs cannot be represented in Anthropic Messages"
                .to_string(),
        ));
    }

    let message = &choice.message;
    if message.function_call.is_some() {
        return Err(TranslationError::InvalidPayload(
                "deprecated Chat response function_call cannot be translated to Anthropic Messages because it has no tool_call_id"
                    .to_string(),
            ));
    }
    let mut content = Vec::new();
    if let Some(reasoning_content) = reasoning_content {
        let (visible_reasoning, continuation) =
            ContinuationEnvelope::split_chat_reasoning_content(reasoning_content)?;
        if let Some(continuation) = continuation {
            for block in continuation {
                match block {
                    Continuation::Thinking {
                        thinking,
                        signature,
                    } if !signature.is_empty() => {
                        content.push(ContentBlock::Thinking(ThinkingBlock {
                            thinking,
                            signature,
                        }));
                    }
                    Continuation::Thinking { .. } => {
                        return Err(TranslationError::InvalidPayload(
                            "Anthropic thinking continuation is missing its signature".to_string(),
                        ));
                    }
                    Continuation::RedactedThinking { data } => {
                        content.push(ContentBlock::RedactedThinking(RedactedThinkingBlock {
                            data,
                        }));
                    }
                }
            }
        }
        if !visible_reasoning.is_empty() {
            scope.dropped(
                "unsigned Chat reasoning_content",
                "Anthropic thinking blocks require a provider signature",
            );
        }
    }

    let has_content = message
        .content
        .as_non_null()
        .is_some_and(|text| !text.is_empty());
    let refusal = message
        .refusal
        .as_non_null()
        .map(String::as_str)
        .filter(|refusal| !refusal.is_empty());
    if has_content && refusal.is_some() {
        return Err(TranslationError::InvalidPayload(
                "Chat completion response contains both content and refusal; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                    .to_string(),
            ));
    }

    if let Some(text) = message
        .content
        .as_non_null()
        .filter(|text| !text.is_empty())
    {
        content.push(ContentBlock::Text(text_block(text)));
    }
    if let Some(refusal) = refusal {
        content.push(ContentBlock::Text(text_block(refusal)));
    }
    if let Some(tool_calls) = message.tool_calls.as_ref() {
        for tool_call in tool_calls {
            content.push(ContentBlock::ToolUse(tool_call.try_into()?));
        }
    }

    if content.is_empty() {
        return Err(TranslationError::InvalidPayload(
                "Chat completion response has no Anthropic-representable content, refusal, or function tool calls"
                    .to_string(),
            ));
    }

    let stop = chat_stop_state(refusal, choice.finish_reason);

    Ok(Message {
        id: format!("msg_{}", chat.id),
        container: None.into(),
        content,
        model: chat.model.clone(),
        role: AnthropicMessageRole::Assistant,
        type_: MessageType::Message,
        stop_details: stop.details.into(),
        stop_reason: stop.reason.into(),
        stop_sequence: stop.sequence.into(),
        usage: chat.usage.as_ref().map(Into::into).unwrap_or_default(),
    })
}

impl TryFrom<&ChatCompletionMessageToolCalls> for ToolUseBlock {
    type Error = TranslationError;

    fn try_from(tool_call: &ChatCompletionMessageToolCalls) -> TranslationResult<Self> {
        match tool_call {
            ChatCompletionMessageToolCalls::Function(call) => {
                let input = serde_json::from_str(&call.function.arguments).map_err(|error| {
                    TranslationError::InvalidPayload(format!(
                        "Chat function tool call arguments are not valid JSON: {error}"
                    ))
                })?;
                Ok(tool_use_block(&call.id, &call.function.name, input))
            }
            ChatCompletionMessageToolCalls::Custom(_) => Err(TranslationError::InvalidPayload(
                "Chat custom tool calls cannot be translated to Anthropic tool_use blocks"
                    .to_string(),
            )),
        }
    }
}

pub(super) struct ChatStopState {
    pub(super) reason: Option<StopReason>,
    pub(super) details: Option<RefusalStopDetails>,
    pub(super) sequence: Option<String>,
}

pub(super) fn chat_stop_state(refusal: Option<&str>, finish_reason: FinishReason) -> ChatStopState {
    let reason = if refusal.is_some() {
        // Chat carries refusal wording in `message.refusal`, while Anthropic
        // identifies refusals with message-level `stop_reason`. Prefer that
        // content semantic over Chat's choice-level `finish_reason`, which is
        // commonly still `stop` for refused turns.
        Some(StopReason::Refusal)
    } else {
        Some(anthropic_stop_reason_from_chat_finish_reason(finish_reason))
    };

    let details = refusal.map(|explanation| RefusalStopDetails {
        type_: RefusalStopDetailsType::Refusal,
        // Chat has no separate refusal metadata field. Use its visible refusal
        // wording as the best available Anthropic stop-details explanation while
        // still keeping the same user-visible text in `content[]`.
        category: None.into(),
        explanation: Some(explanation.to_string()).into(),
    });

    ChatStopState {
        reason,
        details,
        // Chat response choices expose only a broad `finish_reason`; they do
        // not include the concrete stop sequence that ended generation.
        sequence: None,
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
