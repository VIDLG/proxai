//! Basic type conversions and pair-local helpers for
//! `openai_chat_completions -> openai_responses`.

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCalls, CompletionUsage, FinishReason,
};
use crate::protocol::openai::responses::{
    CustomToolCall, IncompleteDetails, IncompleteDetailsReason, InputTokenDetails, OutputItem,
    OutputTokenDetails, ResponseUsage, Status,
};
use crate::translation::openai_responses::outbound::function_call_item;

pub(super) fn incomplete_details_from_finish_reason(
    finish_reason: Option<FinishReason>,
) -> Option<IncompleteDetails> {
    match finish_reason {
        Some(FinishReason::Length) => Some(IncompleteDetails {
            reason: Some(IncompleteDetailsReason::MaxOutputTokens),
        }),
        Some(FinishReason::ContentFilter) => Some(IncompleteDetails {
            reason: Some(IncompleteDetailsReason::ContentFilter),
        }),
        _ => None,
    }
}

pub(super) fn responses_status_from_chat_finish_reason(value: FinishReason) -> Status {
    match value {
        FinishReason::Length | FinishReason::ContentFilter => Status::Incomplete,
        FinishReason::Stop | FinishReason::ToolCalls | FinishReason::FunctionCall => {
            Status::Completed
        }
    }
}

impl From<&CompletionUsage> for ResponseUsage {
    fn from(usage: &CompletionUsage) -> Self {
        Self {
            input_tokens: usage.prompt_tokens,
            input_tokens_details: InputTokenDetails {
                cached_tokens: usage
                    .prompt_tokens_details
                    .and_then(|details| details.cached_tokens)
                    .unwrap_or_default(),
            },
            output_tokens: usage.completion_tokens,
            output_tokens_details: OutputTokenDetails {
                reasoning_tokens: usage
                    .completion_tokens_details
                    .and_then(|details| details.reasoning_tokens)
                    .unwrap_or_default(),
            },
            total_tokens: usage.total_tokens,
        }
    }
}

impl From<&ChatCompletionMessageToolCalls> for OutputItem {
    fn from(tool_call: &ChatCompletionMessageToolCalls) -> Self {
        match tool_call {
            ChatCompletionMessageToolCalls::Function(call) => {
                function_call_item(&call.id, &call.function.name, &call.function.arguments)
            }
            ChatCompletionMessageToolCalls::Custom(call) => Self::CustomToolCall(CustomToolCall {
                id: Some(call.id.clone()),
                call_id: call.id.clone(),
                name: call.custom.name.clone(),
                input: call.custom.input.clone(),
                namespace: None,
            }),
        }
    }
}
