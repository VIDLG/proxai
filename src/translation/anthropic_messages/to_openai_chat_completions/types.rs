use crate::protocol::anthropic::messages::{MessageDeltaUsage, StopReason, ToolUseBlock, Usage};
use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls, CompletionUsage, FinishReason,
    FunctionCall, PromptTokensDetails,
};
use crate::translation::{TranslationError, TranslationResult};

impl TryFrom<&ToolUseBlock> for ChatCompletionMessageToolCalls {
    type Error = TranslationError;

    fn try_from(block: &ToolUseBlock) -> TranslationResult<Self> {
        // Anthropic `tool_use` carries a named JSON input object, which matches
        // OpenAI Chat function tool calls. Chat `custom_tool` is for freeform
        // custom tool input strings, so there is no reliable signal here to use
        // `ChatCompletionMessageToolCalls::Custom`.
        Ok(Self::Function(ChatCompletionMessageToolCall {
            id: block.id.clone(),
            function: FunctionCall {
                name: block.name.clone(),
                arguments: serde_json::to_string(&block.input)?,
            },
        }))
    }
}

impl From<&Usage> for CompletionUsage {
    fn from(usage: &Usage) -> Self {
        completion_usage_from_anthropic(
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_input_tokens,
        )
    }
}

impl From<MessageDeltaUsage> for CompletionUsage {
    fn from(usage: MessageDeltaUsage) -> Self {
        completion_usage_from_anthropic(
            usage.input_tokens.unwrap_or(0),
            usage.output_tokens,
            usage.cache_read_input_tokens,
        )
    }
}

fn completion_usage_from_anthropic(
    input_tokens: u32,
    output_tokens: u32,
    cache_read_input_tokens: Option<u32>,
) -> CompletionUsage {
    CompletionUsage {
        prompt_tokens: input_tokens,
        completion_tokens: output_tokens,
        total_tokens: input_tokens.saturating_add(output_tokens),
        prompt_tokens_details: cache_read_input_tokens.map(|cached_tokens| {
            // Anthropic cache-read input tokens are the closest equivalent to
            // OpenAI Chat prompt cached tokens.
            PromptTokensDetails {
                audio_tokens: None,
                cached_tokens: Some(cached_tokens),
            }
        }),
        // Anthropic usage has no completion-side token breakdown for Chat's
        // reasoning/audio/prediction detail fields.
        completion_tokens_details: None,
    }
}

impl From<StopReason> for FinishReason {
    fn from(stop_reason: StopReason) -> Self {
        match stop_reason {
            // Chat has no dedicated refusal finish reason; a refusal is still a
            // terminal assistant turn rather than a tool-call request.
            StopReason::EndTurn | StopReason::StopSequence | StopReason::Refusal => Self::Stop,
            StopReason::MaxTokens => Self::Length,
            // OpenAI Chat has no `pause_turn` finish reason. Treat it like
            // `tool_use` so clients can continue the turn with follow-up action.
            StopReason::ToolUse | StopReason::PauseTurn => Self::ToolCalls,
        }
    }
}
