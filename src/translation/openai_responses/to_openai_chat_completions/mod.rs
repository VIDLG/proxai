//! `openai_responses -> openai_chat_completions` translation.

mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::openai::chat_completions::{
    CreateChatCompletionRequest, CreateChatCompletionResponse,
};
use crate::protocol::openai::responses::{Response, ResponseCreateParams};
use crate::translation::streaming::translate_sse_stream;
use crate::translation::{TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request =
        json::from_value::<ResponseCreateParams>(payload, "OpenAI Responses request payload")?;
    let translated: CreateChatCompletionRequest = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_response(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::ChatCompletionStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_response(payload: Value) -> TranslationResult<Value> {
    let response = json::from_value::<Response>(&payload, "OpenAI Responses response payload")?;
    let translated: CreateChatCompletionResponse = (&response).try_into()?;
    Ok(serde_json::to_value(translated)?)
}
