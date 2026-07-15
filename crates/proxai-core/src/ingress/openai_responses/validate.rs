use serde_json::Value;

use super::IngressError;

pub(super) fn require_model(payload: &Value) -> Result<String, IngressError> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
        .ok_or(IngressError::MissingModel {
            protocol: crate::protocol::RequestProtocol::OpenaiResponses,
        })
}
