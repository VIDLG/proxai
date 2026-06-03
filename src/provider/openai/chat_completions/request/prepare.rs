use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::protocol::openai::chat_completions::RequestProjection;

use super::summary::RequestSummary;

#[derive(Debug, Clone)]
pub(crate) struct PreparedProviderRequest {
    pub(crate) body: Vec<u8>,
    pub(crate) projection: RequestProjection,
    pub(crate) summary: RequestSummary,
}

pub(crate) fn prepare_provider_request(
    payload: &Value,
    request_model: &str,
    upstream_model: &str,
) -> Result<PreparedProviderRequest, InternalError> {
    let mut payload = payload.clone();
    if upstream_model != request_model {
        if let Some(model) = payload.get_mut("model") {
            *model = Value::String(upstream_model.to_string());
        }
    }

    let projection = RequestProjection::from_payload(&payload)?;
    let summary = RequestSummary::from(&projection);
    let body = serde_json::to_vec(&payload)?;

    Ok(PreparedProviderRequest {
        body,
        projection,
        summary,
    })
}
