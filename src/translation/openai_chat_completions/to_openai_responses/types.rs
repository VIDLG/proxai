//! Basic type conversions and pair-local helpers for
//! `openai_chat_completions -> openai_responses`.

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCalls, CompletionUsage, FinishReason,
};
use crate::protocol::openai::responses::{
    CustomToolCall, FunctionToolCall, IncompleteDetails, InputTokenDetails, OutputItem,
    OutputStatus, OutputTokenDetails, ResponseUsage, Status,
};

/// Normalize a Chat Completions id into a Responses-shaped id.
///
/// Pair-local naming convention, not a protocol conversion: it just makes
/// sure the id starts with `resp_` so downstream consumers recognize it.
pub(super) fn response_id(chat_id: &str) -> String {
    if chat_id.starts_with("resp_") {
        chat_id.to_string()
    } else {
        format!("resp_{chat_id}")
    }
}

pub(super) fn incomplete_details_from_finish_reason(
    finish_reason: Option<FinishReason>,
) -> Option<IncompleteDetails> {
    match finish_reason {
        Some(FinishReason::Length) => Some(IncompleteDetails {
            reason: "max_output_tokens".to_string(),
        }),
        Some(FinishReason::ContentFilter) => Some(IncompleteDetails {
            reason: "content_filter".to_string(),
        }),
        _ => None,
    }
}

impl From<FinishReason> for Status {
    fn from(value: FinishReason) -> Self {
        match value {
            FinishReason::Length | FinishReason::ContentFilter => Self::Incomplete,
            FinishReason::Stop | FinishReason::ToolCalls | FinishReason::FunctionCall => {
                Self::Completed
            }
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
                Self::FunctionCall(FunctionToolCall {
                    id: Some(call.id.clone()),
                    call_id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments: call.function.arguments.clone(),
                    status: Some(OutputStatus::Completed),
                    namespace: None,
                })
            }
            ChatCompletionMessageToolCalls::Custom(call) => Self::CustomToolCall(CustomToolCall {
                id: Some(call.id.clone()),
                call_id: call.id.clone(),
                name: call.custom_tool.name.clone(),
                input: call.custom_tool.input.clone(),
                namespace: None,
            }),
        }
    }
}
