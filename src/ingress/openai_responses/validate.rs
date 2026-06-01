use serde_json::Value;

use crate::error::{RequestError, Result};

pub(super) fn require_model(payload: &Value) -> Result<String, RequestError> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            RequestError::Invalid(
                "OpenAI Responses requests must include a non-empty `model`.".to_string(),
            )
        })
}
