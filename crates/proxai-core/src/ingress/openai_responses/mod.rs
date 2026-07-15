mod normalize;
mod validate;

use serde_json::Value;

use crate::protocol::RequestProtocol;

use super::{IngressError, PreparedInboundRequest};

pub(super) fn prepare_openai_responses_request(
    payload: Value,
) -> Result<PreparedInboundRequest, IngressError> {
    let normalized_payload = normalize::normalize_payload(payload);
    let model = validate::require_model(&normalized_payload)?;

    Ok(PreparedInboundRequest::new(
        RequestProtocol::OpenaiResponses,
        normalized_payload,
        model,
    ))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod translation_tests;
