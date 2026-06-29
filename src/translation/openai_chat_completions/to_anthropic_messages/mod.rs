//! `openai_chat_completions -> anthropic_messages` translation.

mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;

use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase};
use crate::protocol::openai::chat_completions::{
    CreateChatCompletionRequest, CreateChatCompletionResponse,
};
use crate::translation::TranslationResult;

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<CreateChatCompletionRequest>(payload.clone())?;
    let translated: MessageCreateParamsBase = (&request).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let response = serde_json::from_value::<CreateChatCompletionResponse>(payload)?;
    let translated: Message = (&response).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    crate::translation::streaming::translate_sse_stream(
        input,
        streaming::MessagesStreamTranslator::default(),
    )
}

#[cfg(test)]
mod request_tests;
#[cfg(test)]
mod response_tests;
