use super::RequestTranslationPlan;
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[test]
fn returns_no_conversion_for_same_protocol_paths() {
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::OpenaiResponses,
        ),
        RequestTranslationPlan::NoConversion
    );
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::AnthropicMessages,
            ProviderProtocol::AnthropicMessages,
        ),
        RequestTranslationPlan::NoConversion
    );
}

#[test]
fn maps_supported_cross_protocol_paths() {
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::AnthropicMessages,
        ),
        RequestTranslationPlan::OpenaiResponsesToAnthropicMessages
    );
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::AnthropicMessages,
            ProviderProtocol::OpenaiResponses,
        ),
        RequestTranslationPlan::AnthropicMessagesToOpenaiResponses
    );
}

#[test]
fn marks_chat_completions_as_unsupported_for_now() {
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::OpenaiChatCompletions,
            ProviderProtocol::OpenaiResponses,
        ),
        RequestTranslationPlan::Unsupported
    );
    assert_eq!(
        RequestTranslationPlan::for_request_to_provider(
            RequestProtocol::OpenaiChatCompletions,
            ProviderProtocol::AnthropicMessages,
        ),
        RequestTranslationPlan::Unsupported
    );
}

#[test]
fn exposes_protocol_and_module_metadata() {
    let plan = RequestTranslationPlan::OpenaiResponsesToAnthropicMessages;
    assert_eq!(
        plan.inbound_request_protocol(),
        Some(RequestProtocol::OpenaiResponses)
    );
    assert_eq!(
        plan.provider_protocol(),
        Some(ProviderProtocol::AnthropicMessages)
    );
    assert_eq!(
        plan.module_hint(),
        Some("translation/openai_responses/to_anthropic_messages")
    );
}
