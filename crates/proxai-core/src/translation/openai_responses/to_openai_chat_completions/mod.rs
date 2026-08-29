//! `openai_responses -> openai_chat_completions` translation.
//!
//! Request translation lives in `request`, non-streaming response in
//! `response`, streaming in `streaming`.

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

use self::request::ChatRequestProjection;
use self::response::ChatResponseProjection;

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request =
        deserialize_value::<CreateResponseRequest>(payload, "OpenAI Responses request payload")?;
    let ChatRequestProjection {
        request,
        extensions,
    } = request::translate_request(&request, scope)?;
    let mut payload = serde_json::to_value(request)?;
    extensions.apply(&mut payload)?;
    Ok(payload)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let response = deserialize_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let ChatResponseProjection {
        response,
        reasoning,
    } = response::translate_response_payload(&response, scope)?;
    let mut payload = serde_json::to_value(response)?;
    inject_response_reasoning(&mut payload, reasoning)?;
    Ok(payload)
}
