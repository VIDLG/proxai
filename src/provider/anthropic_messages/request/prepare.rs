use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::protocol::anthropic::messages::MessageCreateParamsBase;

use super::summary::RequestSummary;

#[derive(Debug, Clone)]
pub(crate) struct PreparedForwardedRequest {
    pub(crate) body: Vec<u8>,
    pub(crate) projection: MessageCreateParamsBase,
    pub(crate) summary: RequestSummary,
}

pub(crate) fn prepare_forwarded_request(
    payload: &Value,
    request_model: &str,
    upstream_model: &str,
) -> Result<PreparedForwardedRequest, InternalError> {
    let mut payload = payload.clone();
    if upstream_model != request_model {
        if let Some(model) = payload.get_mut("model") {
            *model = Value::String(upstream_model.to_string());
        }
    }

    let projection = serde_json::from_value::<MessageCreateParamsBase>(payload.clone())?;
    let summary = RequestSummary::from(&projection);

    Ok(PreparedForwardedRequest {
        body: serde_json::to_vec(&payload)?,
        projection,
        summary,
    })
}
