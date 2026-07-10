//! Basic type conversions and pair-local helpers for
//! `openai_responses -> openai_chat_completions`.

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls, CompletionTokensDetails,
    CompletionUsage, ImageDetail as ChatImageDetail, PromptTokensDetails,
};
use crate::protocol::openai::responses::{
    CustomToolCall, FunctionToolCall, ImageDetail as ResponsesImageDetail, ResponseUsage,
};

/// Normalize a Responses id into a Chat-shaped id.
///
/// Pair-local naming convention, not a protocol conversion: it just makes the
/// id start with `chatcmpl_` so downstream consumers recognize it.
pub(super) fn chat_id(response_id: &str) -> String {
    if response_id.starts_with("chatcmpl_") {
        response_id.to_string()
    } else {
        format!("chatcmpl_{response_id}")
    }
}

impl From<ResponsesImageDetail> for ChatImageDetail {
    fn from(value: ResponsesImageDetail) -> Self {
        match value {
            ResponsesImageDetail::Auto => Self::Auto,
            ResponsesImageDetail::Low => Self::Low,
            ResponsesImageDetail::High => Self::High,
            ResponsesImageDetail::Original => Self::Original,
        }
    }
}

/// Resolve a tool-call identifier, preferring the explicit `call_id` (which
/// is the wire-level identifier used in tool_call/tool message pairing) and
/// falling back to the optional item `id`.
fn tool_call_id(id: &Option<String>, call_id: &str) -> String {
    if !call_id.is_empty() {
        call_id.to_string()
    } else {
        id.clone().unwrap_or_else(|| "tool_call".to_string())
    }
}

impl From<&ResponseUsage> for CompletionUsage {
    fn from(usage: &ResponseUsage) -> Self {
        Self {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            prompt_tokens_details: (usage.input_tokens_details.cached_tokens > 0).then_some(
                PromptTokensDetails {
                    audio_tokens: None,
                    cached_tokens: Some(usage.input_tokens_details.cached_tokens),
                },
            ),
            completion_tokens_details: (usage.output_tokens_details.reasoning_tokens > 0)
                .then_some(CompletionTokensDetails {
                    accepted_prediction_tokens: None,
                    audio_tokens: None,
                    reasoning_tokens: Some(usage.output_tokens_details.reasoning_tokens),
                    rejected_prediction_tokens: None,
                }),
        }
    }
}

impl From<&FunctionToolCall> for ChatCompletionMessageToolCalls {
    fn from(call: &FunctionToolCall) -> Self {
        Self::Function(ChatCompletionMessageToolCall {
            id: tool_call_id(&call.id, &call.call_id),
            function: crate::protocol::openai::chat_completions::FunctionCall {
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            },
        })
    }
}

impl From<&CustomToolCall> for ChatCompletionMessageToolCalls {
    fn from(call: &CustomToolCall) -> Self {
        Self::Custom(
            crate::protocol::openai::chat_completions::ChatCompletionMessageCustomToolCall {
                id: call.id.clone(),
                custom_tool: crate::protocol::openai::chat_completions::CustomTool {
                    name: call.name.clone(),
                    input: call.input.clone(),
                },
            },
        )
    }
}
