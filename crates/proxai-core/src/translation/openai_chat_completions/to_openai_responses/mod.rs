//! openai_chat_completions -> openai_responses translation.

mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::ChatToResponsesStreaming;

use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::openai::chat_completions::{
    CreateChatCompletionRequest, CreateChatCompletionResponse,
};

use crate::translation::openai_chat_completions::compatibility::{
    ChatRequestExtensions, response_reasoning,
};
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let extensions = ChatRequestExtensions::extract(payload)?;
    let request = deserialize_value::<CreateChatCompletionRequest>(
        payload,
        "OpenAI Chat Completions request payload",
    )?;
    let translated = request::translate_request(&request, &extensions, scope)?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let reasoning = response_reasoning(&payload).map_err(TranslationError::InvalidPayload)?;
    let chat = deserialize_value::<CreateChatCompletionResponse>(
        &payload,
        "OpenAI Chat Completions response payload",
    )?;
    let translated = response::translate_response(&chat, reasoning.as_deref(), scope)?;
    Ok(serde_json::to_value(translated)?)
}
