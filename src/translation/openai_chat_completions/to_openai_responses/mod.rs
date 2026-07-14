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

use crate::translation::streaming::{StreamTranslationFailureSink, translate_sse_stream};
use crate::translation::{TranslationError, TranslationResult, json};

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let extensions =
        crate::translation::openai_chat_completions::compatibility::ChatRequestExtensions::extract(
            payload,
        )?;
    let request = json::from_value::<CreateChatCompletionRequest>(
        payload,
        "OpenAI Chat Completions request payload",
    )?;
    let translated = request::translate_request(&request, &extensions)?;
    Ok(serde_json::to_value(translated)?)
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
    let reasoning =
        crate::translation::openai_chat_completions::compatibility::response_reasoning(&payload)
            .map_err(TranslationError::InvalidPayload)?;
    let chat = json::from_value::<CreateChatCompletionResponse>(
        &payload,
        "OpenAI Chat Completions response payload",
    )?;
    let translated = response::translate_response(&chat, reasoning.as_deref())?;
    Ok(serde_json::to_value(translated)?)
}
