//! Streaming outbound helpers for target OpenAI Chat Completions.
//!
//! These helpers build protocol-native Chat Completions stream chunks from
//! already-decided stream identity, delta, finish reason, and usage values.

use crate::protocol::openai::chat_completions::{
    ChatChoiceStream, ChatCompletionMessageToolCallChunk, ChatCompletionStreamResponseDelta,
    CompletionUsage, CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream,
    FunctionType, Role,
};
use crate::translation::streaming::StreamIdentity;

use super::CHAT_COMPLETION_CHUNK_OBJECT;

pub(crate) fn assistant_role_delta() -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        role: Some(Role::Assistant),
        ..Default::default()
    }
}

pub(crate) fn text_delta(text: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        content: Some(text),
        ..Default::default()
    }
}

pub(crate) fn reasoning_delta(reasoning: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        reasoning_content: Some(reasoning),
        ..Default::default()
    }
}

pub(crate) fn refusal_delta(refusal: String) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        refusal: Some(refusal),
        ..Default::default()
    }
}

pub(crate) fn tool_call_start_delta(
    index: u32,
    id: String,
    name: String,
    type_: Option<FunctionType>,
) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: Some(id),
            r#type: type_,
            function: Some(FunctionCallStream {
                name: Some(name),
                arguments: Some(String::new()),
            }),
        }]),
        ..Default::default()
    }
}

pub(crate) fn tool_arguments_delta(
    index: u32,
    arguments: String,
) -> ChatCompletionStreamResponseDelta {
    ChatCompletionStreamResponseDelta {
        tool_calls: Some(vec![ChatCompletionMessageToolCallChunk {
            index,
            id: None,
            r#type: None,
            function: Some(FunctionCallStream {
                name: None,
                arguments: Some(arguments),
            }),
        }]),
        ..Default::default()
    }
}

pub(crate) fn chat_choice_chunk(
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
        object: CHAT_COMPLETION_CHUNK_OBJECT.to_string(),
        usage: None,
    }
}

pub(crate) fn chat_usage_chunk(
    identity: &StreamIdentity,
    usage: CompletionUsage,
) -> CreateChatCompletionStreamResponse {
    CreateChatCompletionStreamResponse {
        id: identity.id().to_string(),
        choices: Vec::new(),
        created: 0,
        model: identity.model().to_string(),
        service_tier: None,
        object: CHAT_COMPLETION_CHUNK_OBJECT.to_string(),
        usage: Some(usage),
    }
}
