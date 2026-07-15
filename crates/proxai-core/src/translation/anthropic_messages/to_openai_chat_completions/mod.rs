//! `anthropic_messages -> openai_chat_completions` response translation.

mod citations;
mod request;
mod response;
mod streaming;
mod types;

pub(crate) use streaming::AnthropicToChatStreaming;

use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};

use crate::translation::openai_chat_completions::compatibility::inject_response_reasoning;
use crate::translation::{TranslationResult, TranslationScope};

pub(crate) fn translate_request_payload(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let request = deserialize_value::<MessageCreateParamsBase>(
        payload,
        "Anthropic Messages request payload",
    )?;
    let (translated, extensions) = request::translate_request(request, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    extensions.apply(&mut payload)?;
    Ok(payload)
}

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let message = deserialize_value::<Message>(&payload, "Anthropic Messages response payload")?;
    let (translated, reasoning) = response::translate_response(&message, scope)?;
    let mut payload = serde_json::to_value(translated)?;
    inject_response_reasoning(&mut payload, reasoning)?;
    Ok(payload)
}
