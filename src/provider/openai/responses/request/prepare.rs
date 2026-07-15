use serde_json::Value;

use crate::error::{InternalError, Result};
use crate::observe::{ObserveContext, RequestInfoParseFailure};
use crate::protocol::openai_responses::RequestProjection;

use super::projection::{adapt_payload_for_projection, project_payload};
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
    obs: &ObserveContext,
) -> Result<PreparedProviderRequest, InternalError> {
    let projection = project_payload_observed(payload, obs);
    let summary = RequestSummary::from(&projection);

    Ok(PreparedProviderRequest {
        body,
        projection,
        summary,
    })
}

fn project_payload_observed(payload: &Value, obs: &ObserveContext) -> RequestProjection {
    match project_payload(payload) {
        Ok(projection) => projection,
        Err(error) => {
            let adapted = adapt_payload_for_projection(payload);
            obs.observe_request_info_parse_failure(RequestInfoParseFailure {
                normalized_payload: payload,
                request_info_parse_payload: &adapted,
                error: &error,
            });
            RequestProjection::default()
        }
    }
}
