use serde_json::Value;

use crate::error::{RequestError, Result};
use crate::protocol::anthropic::messages::{MessageCreateParamsBase, ThinkingConfigParam};
use tracing::warn;

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
    if let Some(ThinkingConfigParam::Enabled(thinking)) = parsed.thinking.as_ref() {
        warn!(
            event = "anthropic_legacy_thinking_budget",
            model = %parsed.model,
            budget_tokens = thinking.budget_tokens,
            "accepted Anthropic legacy thinking.type=enabled budget_tokens; prefer output_config.effort or thinking.type=adaptive"
        );
    }

    Ok(PreparedAnthropicMessagesRequest {
        normalized_payload: payload,
        model: parsed.model,
    })
}

#[cfg(test)]
mod tests;
