use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::observe::ObserveContext;
use crate::protocol::openai_responses::RequestProjection;

use super::projection::project_payload;
use super::summary::RequestSummary;

#[derive(Debug, Clone)]
pub(crate) struct PreparedProviderRequest {
    pub(crate) body: Vec<u8>,
    pub(crate) projection: RequestProjection,
    pub(crate) summary: RequestSummary,
}

pub(crate) fn prepare_provider_request(
    payload: &Value,
    obs: Option<&ObserveContext>,
    request_model: &str,
    upstream_model: &str,
) -> Result<PreparedProviderRequest, InternalError> {
    let projection = project_payload(payload, obs).unwrap_or_default();
    let summary = RequestSummary::from(&projection);

    let mut payload = payload.clone();
    if upstream_model != request_model
        && let Some(model) = payload.get_mut("model")
    {
        *model = Value::String(upstream_model.to_string());
    }
    let body = serde_json::to_vec(&payload)?;

    Ok(PreparedProviderRequest {
        body,
        projection,
        summary,
    })
}
