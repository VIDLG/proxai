use crate::protocol::anthropic::messages::{StopReason, Usage};
use crate::protocol::openai::chat_completions::{CompletionUsage, FinishReason};

impl From<FinishReason> for StopReason {
    fn from(reason: FinishReason) -> Self {
        match reason {
            FinishReason::Stop => Self::EndTurn,
            FinishReason::Length => Self::MaxTokens,
            FinishReason::ToolCalls | FinishReason::FunctionCall => Self::ToolUse,
            FinishReason::ContentFilter => Self::Refusal,
        }
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
