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
use crate::translation::streaming::translate_sse_stream;
use crate::translation::{TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = json::from_value::<CreateChatCompletionRequest>(
        payload,
        "OpenAI Chat Completions request payload",
    )?;
    let translated: ResponseCreateParams = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::ResponsesStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let chat = json::from_value::<CreateChatCompletionResponse>(
        &payload,
        "OpenAI Chat Completions response payload",
    )?;
    let translated: Response = (&chat).try_into()?;
    Ok(serde_json::to_value(translated)?)
}
