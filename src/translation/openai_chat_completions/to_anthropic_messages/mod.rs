//! `openai_chat_completions -> anthropic_messages` translation.

mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::ChatToAnthropicStreaming;

use serde_json::Value;

use crate::protocol::openai::chat_completions::{
    CreateChatCompletionRequest, CreateChatCompletionResponse,
};

use crate::translation::{TranslationError, TranslationResult, TranslationScope, json};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let extensions =
        crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions::extract(
            payload,
        )?;
    let request = json::from_value::<CreateChatCompletionRequest>(
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
    let reasoning =
        crate::translation::openai_chat_completions::compatibility::response_reasoning(&payload)
            .map_err(TranslationError::InvalidPayload)?;
    let response = json::from_value::<CreateChatCompletionResponse>(
        &payload,
        "OpenAI Chat Completions response payload",
    )?;
    let translated = response::translate_response(&response, reasoning.as_deref(), scope)?;
    Ok(serde_json::to_value(translated)?)
}
