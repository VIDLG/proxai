//! `anthropic_messages -> openai_chat_completions` response translation.

mod citations;
mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};

use crate::translation::streaming::translate_sse_stream;
use crate::translation::{TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request =
        json::from_value::<MessageCreateParamsBase>(payload, "Anthropic Messages request payload")?;
    let (translated, extensions) = request::translate_request(request)?;
    let mut payload = serde_json::to_value(translated)?;
    extensions.apply(&mut payload)?;
    Ok(payload)
}

pub(crate) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::ChatCompletionStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let message = json::from_value::<Message>(&payload, "Anthropic Messages response payload")?;
    let (translated, reasoning) = response::translate_response(&message)?;
    let mut payload = serde_json::to_value(translated)?;
    crate::translation::openai_chat_completions::compatibility::inject_response_reasoning(
        &mut payload,
        reasoning,
    )?;
    Ok(payload)
}
