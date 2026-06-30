//! `anthropic_messages -> openai_responses` translation.

mod citations;
mod ids;
mod request;
mod response;
mod streaming;
mod types;

use serde_json::Value;

use crate::http_support::ByteStream;
use crate::protocol::anthropic::messages::{Message, MessageCreateParamsBase, StopReason};
use crate::protocol::openai_responses::{IncompleteDetails, Response, ResponseCreateParams};
use crate::translation::TranslationResult;
use crate::translation::streaming::translate_sse_stream;

/// Normalize an Anthropic message id into a Responses-shaped id.
///
/// Pair-local naming convention, not a protocol conversion: it just makes
/// sure the id starts with `resp_` so downstream consumers recognize it.
pub(super) fn response_id(message_id: &str) -> String {
    if message_id.starts_with("resp_") {
        message_id.to_string()
    } else {
        format!("resp_{message_id}")
    }
}

/// Pair-local Responses `incomplete_details.reason` convention.
///
/// The string `"max_output_tokens"` is this pair's chosen wording (matching
/// OpenAI Responses API guidance), not an Anthropic protocol value, so it
/// lives with `response_id` rather than in `types.rs`.
pub(super) fn incomplete_details_from_stop_reason(
    stop_reason: Option<StopReason>,
) -> Option<IncompleteDetails> {
    match stop_reason {
        Some(StopReason::MaxTokens) => Some(IncompleteDetails {
            reason: "max_output_tokens".to_string(),
        }),
        _ => None,
    }
}

pub(crate) fn translate_request_payload(payload: &Value) -> TranslationResult<Value> {
    let request = serde_json::from_value::<MessageCreateParamsBase>(payload.clone())?;
    let translated: ResponseCreateParams = request.try_into()?;
    Ok(serde_json::to_value(translated)?)
}

pub(crate) fn translate_streaming_stream(input: ByteStream) -> ByteStream {
    translate_sse_stream(input, streaming::ResponsesStreamTranslator::default())
}

pub(crate) fn translate_non_streaming_payload(payload: Value) -> TranslationResult<Value> {
    let message = serde_json::from_value::<Message>(payload)?;
    let translated: Response = (&message).try_into()?;
    Ok(serde_json::to_value(translated)?)
}
