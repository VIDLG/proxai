use async_openai::types::chat::CreateChatCompletionRequest;
use serde_json::Value;

use crate::error::{RequestError, Result};

#[derive(Debug)]
pub(crate) struct PreparedOpenaiChatCompletionsRequest {
    pub(crate) normalized_payload: Value,
    pub(crate) model: String,
}

pub(crate) fn prepare_openai_chat_completions_request(
    body: &[u8],
) -> Result<PreparedOpenaiChatCompletionsRequest, RequestError> {
    let payload = serde_json::from_slice::<Value>(body).map_err(|_| {
        RequestError::Invalid(
            "OpenAI Chat Completions requests must be JSON and include a non-empty `model`."
                .to_string(),
        )
    })?;

    let parsed =
        serde_json::from_value::<CreateChatCompletionRequest>(payload.clone()).map_err(|_| {
            RequestError::Invalid(
                "OpenAI Chat Completions requests must include `model` and `messages`.".to_string(),
            )
        })?;
    if parsed.model.trim().is_empty() {
        return Err(RequestError::Invalid(
            "OpenAI Chat Completions requests must include a non-empty `model`.".to_string(),
        ));
    }

    Ok(PreparedOpenaiChatCompletionsRequest {
        normalized_payload: payload,
        model: parsed.model,
    })
}

#[cfg(test)]
mod tests;
