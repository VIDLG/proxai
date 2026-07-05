//! Pure output builders for
//! `openai_responses -> openai_chat_completions` streaming translation.
//!
//! These constructors take already-decided protocol values (identity, deltas,
//! finish reason) and assemble Chat Completions stream response payloads. They
//! own no streaming state.

use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, ChatCompletionStreamResponseDelta,
    CompletionUsage, CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream, Role,
};
use crate::translation::streaming::StreamIdentity;

/// Build a Chat stream chunk carrying one assistant-message delta choice.
pub(super) fn chat_choice_chunk(
    identity: &StreamIdentity,
    delta: ChatCompletionStreamResponseDelta,
    finish_reason: Option<FinishReason>,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: identity.id().to_string(),
        choices: vec![ChatChoiceStream {
            index: 0,
            delta,
            finish_reason,
            logprobs: None,
        }],
        created: 0,
        model: identity.model().to_string(),
        service_tier: None,
        object: "chat.completion.chunk".to_string(),
        usage: None,
    }
}

/// Build a Chat stream chunk carrying only a usage update (`choices: []`),
/// matching OpenAI's `stream_options.include_usage` shape.
pub(super) fn chat_usage_chunk(
    identity: &StreamIdentity,
    usage: CompletionUsage,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: identity.id().to_string(),
        choices: Vec::new(),
        created: 0,
        model: identity.model().to_string(),
        service_tier: None,
        object: "chat.completion.chunk".to_string(),
        usage: Some(usage),
    }
}

/// Initial assistant-message delta emitted for a Responses
/// `response.created` / `response.in_progress`.
///
/// Chat Completions streams open the assistant envelope as a role-only delta;
/// no Responses message-start fields map onto it.
pub(super) fn message_start_delta() -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        role: Some(Role::Assistant),
        ..Default::default()
    }
}

/// Append a Chat content fragment from a Responses `output_text.delta`.
pub(super) fn text_delta(delta: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        content: Some(delta),
        ..Default::default()
    }
}

/// Append a Chat `reasoning_content` fragment from a Responses
/// `reasoning_text.delta` / `reasoning_summary_text.delta`.
pub(super) fn reasoning_delta(delta: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        reasoning_content: Some(delta),
        ..Default::default()
    }
}

/// Open a Chat tool-call stream from a Responses `function_call` output item.
///
/// Starts the Chat arguments stream with an empty string. `None` would
/// serialize as JSON null in the local wire model, while OpenAI-compatible
/// tool argument deltas are string fragments.
pub(super) fn tool_call_start_delta(
    index: u32,
    id: String,
    name: String,
) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: Some(id),
            r#type: None,
            function: Some(FunctionCallStream {
                name: Some(name),
                arguments: Some(String::new()),
            }),
        }]),
        ..Default::default()
    }
}

/// Append a Chat tool-call arguments fragment from a Responses
/// `function_call_arguments.delta`.
pub(super) fn tool_arguments_delta(index: u32, delta: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: None,
            r#type: None,
            function: Some(FunctionCallStream {
                name: None,
                arguments: Some(delta),
            }),
        }]),
        ..Default::default()
    }
}
