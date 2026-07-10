use crate::protocol::anthropic::messages::{MessageDeltaUsage, StopReason, Usage};
use crate::protocol::openai::chat_completions::{CompletionUsage, FinishReason};

pub(super) fn anthropic_stop_reason_from_chat_finish_reason(reason: FinishReason) -> StopReason {
    match reason {
        FinishReason::Stop => StopReason::EndTurn,
        FinishReason::Length => StopReason::MaxTokens,
        FinishReason::ToolCalls | FinishReason::FunctionCall => StopReason::ToolUse,
        FinishReason::ContentFilter => StopReason::Refusal,
    }
}

impl From<&CompletionUsage> for Usage {
    fn from(usage: &CompletionUsage) -> Self {
        Self {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            ..Self::default()
        }
    }
}

impl From<&CompletionUsage> for MessageDeltaUsage {
    fn from(usage: &CompletionUsage) -> Self {
        Self {
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            input_tokens: Some(usage.prompt_tokens),
            output_tokens: usage.completion_tokens,
            output_tokens_details: None,
            server_tool_use: None,
        }
    }
}
