//! `anthropic_messages -> openai_responses` translation.

mod citations;
mod ids;
mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai_responses::{CreateResponseRequest, Response};

use crate::translation::streaming::{StreamTranslationFailureSink, translate_sse_stream};
use crate::translation::{TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request =
        json::from_value::<MessageCreateParamsBase>(payload, "Anthropic Messages request payload")?;
    let translated: CreateResponseRequest = request.try_into()?;
    Ok(serde_json::to_value(translated)?)
}

#[cfg(test)]
pub(crate) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_streaming_response_with_failure_sink(input, StreamTranslationFailureSink::default())
}

pub(crate) fn translate_streaming_response_with_failure_sink(
    input: ByteStream,
    failure_sink: StreamTranslationFailureSink,
) -> ByteStream {
    translate_sse_stream(
        input,
        streaming::ResponsesStreamTranslator::default(),
        failure_sink,
    )
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let message = json::from_value::<Message>(&payload, "Anthropic Messages response payload")?;
    let translated: Response = (&message).try_into()?;
    Ok(serde_json::to_value(translated)?)
}
