use proxai_core::observe::{
    IngressObservation, Observation, ProviderObservation, ProviderRequestAdaptation,
};

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
        Observation::Provider(ProviderObservation::RequestAdapted {
            protocol,
            adaptation:
                ProviderRequestAdaptation::OpenaiResponsesOutputFieldsRemoved {
                    status_removed,
                    reasoning_content_removed,
                },
        }) => tracing::trace!(
            provider_protocol = %protocol,
            phase = "request",
            adaptation = "openai_responses_output_fields_removed",
            status_removed,
            reasoning_content_removed,
            "provider compatibility observation"
        ),
        Observation::Provider(ProviderObservation::ResponseAdapted {
            protocol,
            phase,
            adaptation,
        }) => tracing::trace!(
            provider_protocol = %protocol,
            phase = %phase,
            adaptation = %adaptation,
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
