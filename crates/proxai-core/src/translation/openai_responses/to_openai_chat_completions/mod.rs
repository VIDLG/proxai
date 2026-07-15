//! `openai_responses -> openai_chat_completions` translation.

mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::ResponsesToChatStreaming;

use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::openai::responses::{CreateResponseRequest, Response};

use crate::translation::openai_chat_completions::compatibility::inject_response_reasoning;
use crate::translation::{TranslationResult, TranslationScope};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request =
        deserialize_value::<CreateResponseRequest>(payload, "OpenAI Responses request payload")?;
    let (translated, extensions) = request::translate_request(&request, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    extensions.apply(&mut payload)?;
    Ok(payload)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let response = deserialize_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let (translated, reasoning) = response::translate_response(&response, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    inject_response_reasoning(&mut payload, reasoning)?;
    Ok(payload)
}
