//! `openai_responses -> anthropic_messages` translation.
//!
//! Request translation lives in `request`, non-streaming response in
//! `response`, streaming in `streaming`.

mod request;
mod response;
pub(super) mod streaming;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai_responses::{CreateResponseRequest, Response};
use crate::translation::streaming::StreamTranslationFailureSink;
use crate::translation::{TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request =
        json::from_value::<CreateResponseRequest>(payload, "OpenAI Responses request payload")?;
    let translated: MessageCreateParamsBase = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let response = json::from_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let translated: Message = response::translate_response_payload(&response);
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_response_with_failure_sink(
    input: ByteStream,
    failure_sink: StreamTranslationFailureSink,
) -> ByteStream {
    streaming::translate_streaming_response_with_failure_sink(input, failure_sink)
}
