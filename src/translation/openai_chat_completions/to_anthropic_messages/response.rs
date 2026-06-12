//! Non-streaming response conversion for `openai_chat_completions -> anthropic_messages`.

use crate::protocol::anthropic::messages::{
    ContentBlock, DirectCaller, Message, MessageType, RefusalStopDetails, RefusalStopDetailsType,
    Role as AnthropicRole, StopReason, TextBlock, ToolCaller, ToolUseBlock,
};
use crate::protocol::openai::chat_completions::{
    ChatChoice, ChatCompletionMessageToolCalls, CreateChatCompletionResponse, FinishReason,
    Role as ChatRole,
};
use crate::translation::{TranslationError, TranslationResult};

fn single_representable_response_choice(choices: &[ChatChoice]) -> TranslationResult<&ChatChoice> {
    let choice = match choices {
        [] => {
            return Err(TranslationError::InvalidPayload(
                "Chat completion response has no choices to translate to Anthropic message"
                    .to_string(),
            ));
        }
        [choice] => choice,
        choices => {
            return Err(TranslationError::InvalidPayload(format!(
                "Chat completion response has {} choices; Anthropic message responses can represent exactly one assistant message",
                choices.len()
            )));
        }
    };

    if choice.logprobs.is_some() {
        return Err(TranslationError::InvalidPayload(
            "Chat completion response choice logprobs cannot be represented in Anthropic Messages"
                .to_string(),
        ));
    }
    if choice.message.role != ChatRole::Assistant {
        return Err(TranslationError::InvalidPayload(format!(
            "Chat completion response role {} cannot be represented as an Anthropic assistant message",
            choice.message.role
        )));
    }

    Ok(choice)
}

impl TryFrom<&CreateChatCompletionResponse> for Message {
    type Error = TranslationError;

    fn try_from(chat: &CreateChatCompletionResponse) -> TranslationResult<Self> {
        let choice = single_representable_response_choice(&chat.choices)?;

        let message = &choice.message;
        let mut content = Vec::new();

        let has_content = message
            .content
            .as_ref()
            .is_some_and(|text| !text.is_empty());
        let refusal = message
            .refusal
            .as_deref()
            .filter(|refusal| !refusal.is_empty());
        if has_content && refusal.is_some() {
            return Err(TranslationError::InvalidPayload(
                "Chat completion response contains both content and refusal; Anthropic Messages requires refusal semantics to be represented by message-level stop fields"
                    .to_string(),
            ));
        }

        if let Some(text) = message.content.as_ref().filter(|text| !text.is_empty()) {
            content.push(ContentBlock::Text(TextBlock {
                citations: None,
                text: text.clone(),
            }));
        }
        if let Some(refusal) = refusal {
            content.push(ContentBlock::Text(TextBlock {
                citations: None,
                text: refusal.to_string(),
            }));
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

        Ok(Self {
            id: format!("msg_{}", chat.id),
            container: None,
            content,
            model: chat.model.clone(),
            role: AnthropicRole::Assistant,
            type_: MessageType::Message,
            stop_details: stop.details,
            stop_reason: stop.reason,
            stop_sequence: stop.sequence,
            usage: chat.usage.as_ref().map(Into::into).unwrap_or_default(),
        })
    }
}

impl TryFrom<&ChatCompletionMessageToolCalls> for ToolUseBlock {
    type Error = TranslationError;

    fn try_from(tool_call: &ChatCompletionMessageToolCalls) -> TranslationResult<Self> {
        match tool_call {
            ChatCompletionMessageToolCalls::Function(call) => Ok(Self {
                id: call.id.clone(),
                caller: ToolCaller::Direct(DirectCaller),
                input: serde_json::from_str(&call.function.arguments).map_err(|error| {
                    TranslationError::InvalidPayload(format!(
                        "Chat function tool call arguments are not valid JSON: {error}"
                    ))
                })?,
                name: call.function.name.clone(),
            }),
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

pub(super) fn chat_stop_state(
    refusal: Option<&str>,
    finish_reason: Option<FinishReason>,
) -> ChatStopState {
    let reason = if refusal.is_some() {
        // Chat carries refusal wording in `message.refusal`, while Anthropic
        // identifies refusals with message-level `stop_reason`. Prefer that
        // content semantic over Chat's choice-level `finish_reason`, which is
        // commonly still `stop` for refused turns.
        Some(StopReason::Refusal)
    } else {
        finish_reason.map(Into::into)
    };

    let details = refusal.map(|explanation| RefusalStopDetails {
        type_: RefusalStopDetailsType::Refusal,
        // Chat has no separate refusal metadata field. Use its visible refusal
        // wording as the best available Anthropic stop-details explanation while
        // still keeping the same user-visible text in `content[]`.
        category: None,
        explanation: Some(explanation.to_string()),
    });

    ChatStopState {
        reason,
        details,
        // Chat response choices expose only a broad `finish_reason`; they do
        // not include the concrete stop sequence that ended generation.
        sequence: None,
    }
}
