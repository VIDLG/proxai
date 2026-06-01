use serde_json::Value;

use crate::error::{RequestError, Result};
use crate::protocol::anthropic::messages::MessageCreateParamsBase;

#[derive(Debug)]
pub(crate) struct PreparedAnthropicMessagesRequest {
    pub(crate) normalized_payload: Value,
    pub(crate) model: String,
}

pub(crate) fn prepare_anthropic_messages_request(
    body: &[u8],
) -> Result<PreparedAnthropicMessagesRequest, RequestError> {
    let payload = serde_json::from_slice::<Value>(body).map_err(|_| {
        RequestError::Invalid(
            "Anthropic Messages requests must be JSON and include a non-empty `model`.".to_string(),
        )
    })?;

    let parsed =
        serde_json::from_value::<MessageCreateParamsBase>(payload.clone()).map_err(|_| {
            RequestError::Invalid(
                "Anthropic Messages requests must include `model`, `max_tokens`, and `messages`."
                    .to_string(),
            )
        })?;
    if parsed.model.trim().is_empty() {
        return Err(RequestError::Invalid(
            "Anthropic Messages requests must include a non-empty `model`.".to_string(),
        ));
    }

    Ok(PreparedAnthropicMessagesRequest {
        normalized_payload: payload,
        model: parsed.model,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
