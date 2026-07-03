//! `openai_responses -> anthropic_messages` translation.
//!
//! Request translation lives in `request`, non-streaming response in
//! `response`, streaming in `streaming`.

mod request;
mod response;
pub(super) mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai_responses::{Response, ResponseCreateParams};
use crate::translation::TranslationResult;

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<ResponseCreateParams>(payload.clone())?;
    let translated: MessageCreateParamsBase = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let response = serde_json::from_value::<Response>(payload)?;
    let translated: Message = response::translate_response_payload(&response);
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    streaming::translate_streaming_response(input)
}
