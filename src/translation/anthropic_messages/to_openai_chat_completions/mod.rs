//! `anthropic_messages -> openai_chat_completions` response translation.

mod citations;
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
use crate::translation::sse::translate_sse_stream;

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<MessageCreateParamsBase>(payload.clone())?;
    let translated: CreateChatCompletionRequest = request.try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::AnthropicToChatStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let message = serde_json::from_value::<Message>(payload)?;
    let translated: CreateChatCompletionResponse = (&message).try_into()?;
    Ok(serde_json::to_value(translated)?)
}

#[cfg(test)]
mod tests;
