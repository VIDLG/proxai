//! `anthropic_messages -> openai_responses` translation.

mod citations;
mod ids;
mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::AnthropicToResponsesStreaming;

use serde_json::Value;

use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai_responses::{CreateResponseRequest, Response};

use crate::translation::{TranslationResult, TranslationScope, json};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request =
        json::from_value::<MessageCreateParamsBase>(payload, "Anthropic Messages request payload")?;
    let translated: CreateResponseRequest = request::translate_request(request, scope)?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let message = json::from_value::<Message>(&payload, "Anthropic Messages response payload")?;
    let translated: Response = response::translate_response(&message, scope)?;
    Ok(serde_json::to_value(translated)?)
}
