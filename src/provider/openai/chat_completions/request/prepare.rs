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
    body: Vec<u8>,
) -> Result<PreparedProviderRequest, InternalError> {
    let projection = RequestProjection::from_payload(payload)?;
    let summary = RequestSummary::from(&projection);

    Ok(PreparedProviderRequest {
        body,
        projection,
        summary,
    })
}
