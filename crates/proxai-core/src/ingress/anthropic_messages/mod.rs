use serde_json::Value;

use crate::json::deserialize_value;
use crate::observe::{IngressObservation, Observation, Observer};
use crate::protocol::RequestProtocol;
use crate::protocol::anthropic::messages::{MessageCreateParamsBase, ThinkingConfigParam};

use super::{IngressError, PreparedInboundRequest};

pub(super) fn prepare_anthropic_messages_request(
    payload: Value,
    observer: &dyn Observer,
) -> Result<PreparedInboundRequest, IngressError> {
    let parsed = deserialize_value::<MessageCreateParamsBase>(
        &payload,
        "Anthropic Messages request payload",
    )?;
    if parsed.model.trim().is_empty() {
        return Err(IngressError::MissingModel {
            protocol: RequestProtocol::AnthropicMessages,
        });
    }

    if let Some(ThinkingConfigParam::Enabled(thinking)) = parsed.thinking.as_ref() {
        observer.observe(&Observation::from(
            IngressObservation::AnthropicLegacyThinkingBudget {
                model: parsed.model.clone(),
                budget_tokens: thinking.budget_tokens,
            },
        ));
    }

    Ok(PreparedInboundRequest::new(
        RequestProtocol::AnthropicMessages,
        payload,
        parsed.model,
    ))
}

#[cfg(test)]
mod tests;
