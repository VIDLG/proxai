//! Pair-local protocol-to-protocol conversions shared across the
//! `openai_responses -> openai_chat_completions` pair's `request` / `response` /
//! `streaming` children.
//!
//! Only stateless conversions shared by more than one child belong here.
//! Request-only input field conversions live in `request/types.rs`. Pair-local
//! helpers used by a single child (e.g. response-side id shaping) live inside
//! that child module, not here.

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageCustomToolCall, ChatCompletionMessageToolCall,
    ChatCompletionMessageToolCalls, CompletionTokensDetails, CompletionUsage, CustomTool,
    FinishReason, FunctionCall, PromptTokensDetails, ServiceTier as ChatServiceTier,
};
use crate::protocol::openai::responses::{
    CustomToolCall, FunctionToolCall, ResponseUsage, ServiceTier as ResponsesServiceTier,
};
use crate::translation::openai_responses::stop::ResponsesStopKind;

impl From<ResponsesStopKind> for FinishReason {
    fn from(value: ResponsesStopKind) -> Self {
        match value {
            ResponsesStopKind::EndTurn | ResponsesStopKind::Refusal => Self::Stop,
            ResponsesStopKind::MaxTokens => Self::Length,
            ResponsesStopKind::ToolUse => Self::ToolCalls,
        }
    }
}

impl From<ResponsesServiceTier> for ChatServiceTier {
    fn from(value: ResponsesServiceTier) -> Self {
        match value {
            ResponsesServiceTier::Auto => Self::Auto,
            ResponsesServiceTier::Default => Self::Default,
            ResponsesServiceTier::Flex => Self::Flex,
            ResponsesServiceTier::Scale => Self::Scale,
            ResponsesServiceTier::Priority => Self::Priority,
        }
    }
}

/// Resolve a tool-call identifier, preferring the explicit `call_id`
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
            prompt_tokens_details: (usage.input_tokens_details.cached_tokens > 0
                || usage.input_tokens_details.cache_write_tokens > 0)
                .then_some(PromptTokensDetails {
                    audio_tokens: None,
                    cached_tokens: Some(usage.input_tokens_details.cached_tokens),
                    cache_write_tokens: Some(usage.input_tokens_details.cache_write_tokens),
                }),
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
            function: FunctionCall {
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            },
        })
    }
}

impl From<&CustomToolCall> for ChatCompletionMessageToolCalls {
    fn from(call: &CustomToolCall) -> Self {
        Self::Custom(ChatCompletionMessageCustomToolCall {
            id: call.id.clone().unwrap_or_else(|| call.call_id.clone()),
            custom: CustomTool {
                name: call.name.clone(),
                input: call.input.clone(),
            },
        })
    }
}
