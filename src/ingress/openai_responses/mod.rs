mod normalize;
mod validate;

use serde_json::Value;

use crate::error::{RequestError, Result};

#[derive(Debug)]
pub(crate) struct PreparedOpenaiResponsesRequest {
    pub(crate) normalized_payload: Value,
    pub(crate) model: String,
}

pub(crate) fn prepare_openai_responses_request(
    body: &[u8],
) -> Result<PreparedOpenaiResponsesRequest, RequestError> {
    let payload = serde_json::from_slice::<Value>(body).map_err(|_| {
        RequestError::Invalid(
            "OpenAI Responses requests must be JSON and include a non-empty `model`.".to_string(),
        )
    })?;

    let normalized_payload = normalize::normalize_payload(payload);
    let model = validate::require_model(&normalized_payload)?;

    Ok(PreparedOpenaiResponsesRequest {
        normalized_payload,
        model,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
