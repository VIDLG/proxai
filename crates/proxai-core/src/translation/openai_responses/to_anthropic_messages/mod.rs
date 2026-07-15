//! `openai_responses -> anthropic_messages` translation.
//!
//! Request translation lives in `request`, non-streaming response in
//! `response`, streaming in `streaming`.

mod request;
mod response;
mod streaming;

pub(crate) use streaming::ResponsesToAnthropicStreaming;

use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai_responses::{CreateResponseRequest, Response};

use crate::translation::{TranslationResult, TranslationScope};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request =
        deserialize_value::<CreateResponseRequest>(payload, "OpenAI Responses request payload")?;
    let translated: MessageCreateParamsBase = request::translate_request(&request, scope)?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let response = deserialize_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let translated: Message = response::translate_response_payload(&response, scope);
    Ok(serde_json::to_value(translated)?)
}
