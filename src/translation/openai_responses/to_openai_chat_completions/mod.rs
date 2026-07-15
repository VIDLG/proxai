//! `openai_responses -> openai_chat_completions` translation.

mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::ResponsesToChatStreaming;

use serde_json::Value;

use crate::protocol::openai::responses::{CreateResponseRequest, Response};

use crate::translation::{TranslationResult, TranslationScope, json};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request =
        json::from_value::<CreateResponseRequest>(payload, "OpenAI Responses request payload")?;
    let (translated, extensions) = request::translate_request(&request, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    extensions.apply(&mut payload)?;
    Ok(payload)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let response = json::from_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let (translated, reasoning) = response::translate_response(&response, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    crate::translation::openai_chat_completions::compatibility::inject_response_reasoning(
        &mut payload,
        reasoning,
    )?;
    Ok(payload)
}
