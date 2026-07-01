//! Pure output builders for
//! `anthropic_messages -> openai_chat_completions` streaming translation.
//!
//! These constructors take already-decided protocol values (identity,
//! deltas, finish reason) and assemble Chat Completions stream response
//! payloads. They own no streaming state.

use crate::protocol::anthropic::messages::{MessageDelta, StopReason, ToolUseBlock};
use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, ChatCompletionStreamResponseDelta,
    CompletionUsage, CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream,
    FunctionType, Role,
};
use crate::translation::streaming::StreamIdentity;

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

/// Initial assistant-message delta emitted for an Anthropic `message_start`.
///
/// Chat Completions streams open the assistant envelope as a role-only delta;
/// no message-start fields map onto it.
pub(super) fn message_start_delta() -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        content: None,
        tool_calls: None,
        role: Some(Role::Assistant),
        refusal: None,
        reasoning_content: None,
    }
}

/// Open a Chat tool-call stream from an Anthropic tool-use block.
///
/// Starts the Chat arguments stream with an empty string. `None` would
/// serialize as JSON null in the local wire model, while OpenAI-compatible
/// tool argument deltas are string fragments.
pub(super) fn tool_call_start_delta(
    index: u32,
    block: ToolUseBlock,
) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        content: None,
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: Some(block.id),
            r#type: Some(FunctionType::Function),
            function: Some(FunctionCallStream {
                name: Some(block.name),
                arguments: Some(String::new()),
            }),
        }]),
        role: None,
        refusal: None,
        reasoning_content: None,
    }
}

/// Append a Chat tool-call arguments fragment from an Anthropic
/// `input_json_delta`.
pub(super) fn tool_arguments_delta(
    index: u32,
    partial_json: String,
) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        content: None,
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: None,
            r#type: None,
            function: Some(FunctionCallStream {
                name: None,
                arguments: Some(partial_json),
            }),
        }]),
        role: None,
        refusal: None,
        reasoning_content: None,
    }
}

pub(super) fn chat_terminal_delta(delta: MessageDelta, emitted_text: bool) -> Option<String> {
    // MessageDelta.stop_reason is converted by the caller into Chat's
    // choice-level `finish_reason`; Chat stream deltas have no field for
    // Anthropic `container` or `stop_sequence`.
    //
    // Non-streaming refusal conversion can move final text into
    // `message.refusal` and leave `message.content` empty. Streaming cannot
    // retract text deltas that were already sent without buffering the whole
    // response, so only emit `delta.refusal` when no text content has been
    // streamed yet.
    if emitted_text || !matches!(delta.stop_reason, Some(StopReason::Refusal)) {
        return None;
    }

    let Some(stop_details) = delta.stop_details else {
        return None;
    };

    stop_details.explanation
}
