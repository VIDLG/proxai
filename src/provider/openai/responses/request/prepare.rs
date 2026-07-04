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

pub(crate) fn sanitize_provider_payload(mut payload: Value) -> Value {
    let sanitized = sanitize_response_output_fields_from_input(&mut payload);
    if sanitized.status_removed > 0 || sanitized.reasoning_content_removed > 0 {
        tracing::trace!(
            status_removed = sanitized.status_removed,
            reasoning_content_removed = sanitized.reasoning_content_removed,
            "removed Responses output-only input fields for upstream compatibility"
        );
    }
    payload
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

#[derive(Debug, Default)]
struct SanitizedInputFields {
    status_removed: usize,
    reasoning_content_removed: usize,
}

fn sanitize_response_output_fields_from_input(payload: &mut Value) -> SanitizedInputFields {
    let Some(items) = payload.get_mut("input").and_then(Value::as_array_mut) else {
        return SanitizedInputFields::default();
    };

    let mut sanitized = SanitizedInputFields::default();
    for item in items {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        let Some(item_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        let is_message_or_reasoning = matches!(item_type, "message" | "reasoning");
        let is_reasoning = item_type == "reasoning";
        if is_message_or_reasoning && object.remove("status").is_some() {
            sanitized.status_removed += 1;
        }
        if is_reasoning && object.remove("content").is_some() {
            sanitized.reasoning_content_removed += 1;
        }
    }
    sanitized
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
