use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::protocol::anthropic::messages::MessageCreateParamsBase;

use super::summary::RequestSummary;

#[derive(Debug, Clone)]
pub(crate) struct PreparedProviderRequest {
    pub(crate) body: Vec<u8>,
    pub(crate) projection: MessageCreateParamsBase,
    pub(crate) summary: RequestSummary,
}

pub(crate) fn prepare_provider_request(
    payload: &Value,
    body: Vec<u8>,
) -> Result<PreparedProviderRequest, InternalError> {
    let projection = serde_json::from_value::<MessageCreateParamsBase>(payload.clone())?;
    let summary = RequestSummary::from(&projection);

    Ok(PreparedProviderRequest {
        body,
        projection,
        summary,
    })
}
