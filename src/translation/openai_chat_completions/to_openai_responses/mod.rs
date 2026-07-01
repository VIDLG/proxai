//! openai_chat_completions -> openai_responses translation.

mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::openai::chat_completions::{
    CreateChatCompletionRequest, CreateChatCompletionResponse,
};
use crate::protocol::openai_responses::{Response, ResponseCreateParams};
use crate::translation::TranslationResult;
use crate::translation::streaming::translate_sse_stream;

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<CreateChatCompletionRequest>(payload.clone())?;
    let translated: ResponseCreateParams = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::ResponsesStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(payload)?;
    let translated: Response = (&chat).try_into()?;
    Ok(serde_json::to_value(translated)?)
}
