use proxai_core::observe::{IngressObservation, Observation};

pub(super) fn emit(observation: &Observation) {
    match observation {
        Observation::Ingress(IngressObservation::AnthropicLegacyThinkingBudget {
            model,
            budget_tokens,
        }) => tracing::warn!(
            event = "anthropic_legacy_thinking_budget",
            model,
            budget_tokens,
            "accepted Anthropic legacy thinking.type=enabled budget_tokens; prefer output_config.effort or thinking.type=adaptive"
        ),
        Observation::Provider(observation) => tracing::trace!(
            provider_protocol = %observation.protocol,
            phase = %observation.phase,
            adaptation = %observation.adaptation,
            "provider compatibility observation"
        ),
        Observation::Translation(observation) => tracing::trace!(
            request_protocol = %observation.request_protocol,
            provider_protocol = %observation.provider_protocol,
            phase = ?observation.phase,
            kind = ?observation.kind,
            subject = observation.subject,
            detail = observation.detail,
            "protocol translation observation"
        ),
    }
}
