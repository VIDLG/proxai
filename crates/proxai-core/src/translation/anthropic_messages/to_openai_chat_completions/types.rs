use crate::protocol::anthropic::messages::{
    MessageDeltaUsage, StopReason, TextBlock, TextDelta, ToolUseBlock, Usage,
};
use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionStreamResponseDelta, CompletionUsage, FinishReason, FunctionCall,
    PromptTokensDetails,
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

impl From<TextBlock> for ChatCompletionStreamResponseDelta {
    fn from(block: TextBlock) -> Self {
        Self {
            content: Some(block.text).into(),
            ..Self::default()
        }
    }
}

impl From<TextDelta> for ChatCompletionStreamResponseDelta {
    fn from(delta: TextDelta) -> Self {
        Self {
            content: Some(delta.text).into(),
            ..Self::default()
        }
    }
}

impl From<&Usage> for CompletionUsage {
    fn from(usage: &Usage) -> Self {
        completion_usage_from_anthropic(
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_input_tokens.as_non_null().copied(),
            usage.cache_creation_input_tokens.as_non_null().copied(),
        )
    }
}

impl From<MessageDeltaUsage> for CompletionUsage {
    fn from(usage: MessageDeltaUsage) -> Self {
        completion_usage_from_anthropic(
            usage.input_tokens.as_non_null().copied().unwrap_or(0),
            usage.output_tokens,
            usage.cache_read_input_tokens.into_non_null(),
            usage.cache_creation_input_tokens.into_non_null(),
        )
    }
}

fn completion_usage_from_anthropic(
    input_tokens: u32,
    output_tokens: u32,
    cache_read_input_tokens: Option<u32>,
    cache_write_input_tokens: Option<u32>,
) -> CompletionUsage {
    CompletionUsage {
        prompt_tokens: input_tokens,
        completion_tokens: output_tokens,
        total_tokens: input_tokens.saturating_add(output_tokens),
        prompt_tokens_details: (cache_read_input_tokens.is_some()
            || cache_write_input_tokens.is_some())
        .then_some(PromptTokensDetails {
            audio_tokens: None,
            cached_tokens: cache_read_input_tokens,
            cache_write_tokens: cache_write_input_tokens,
        }),
        // Anthropic usage has no completion-side token breakdown for Chat's
        // reasoning/audio/prediction detail fields.
        completion_tokens_details: None,
    }
}

pub(super) fn chat_finish_reason_from_anthropic_stop_reason(
    stop_reason: StopReason,
) -> FinishReason {
    match stop_reason {
        // Chat has no dedicated refusal finish reason; a refusal is still a
        // terminal assistant turn rather than a tool-call request.
        StopReason::EndTurn | StopReason::StopSequence | StopReason::Refusal => FinishReason::Stop,
        StopReason::MaxTokens => FinishReason::Length,
        // OpenAI Chat has no `pause_turn` finish reason. Treat it like
        // `tool_use` so clients can continue the turn with follow-up action.
        StopReason::ToolUse | StopReason::PauseTurn => FinishReason::ToolCalls,
    }
}
