use crate::protocol::{ProviderProtocol, RequestProtocol};

/// Request translation routing plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequestTranslationPlan {
    NoConversion,
    OpenaiResponsesToAnthropicMessages,
    AnthropicMessagesToOpenaiResponses,
    Unsupported,
}

impl RequestTranslationPlan {
    pub(crate) fn for_request_to_provider(
        request_protocol: RequestProtocol,
        provider_protocol: ProviderProtocol,
    ) -> Self {
        match (request_protocol, provider_protocol) {
            (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
            | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
            | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => {
                Self::NoConversion
            }
            (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
                Self::OpenaiResponsesToAnthropicMessages
            }
            (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
                Self::AnthropicMessagesToOpenaiResponses
            }
            (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions)
            | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses)
            | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages)
            | (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
                Self::Unsupported
            }
        }
    }

    pub(crate) fn inbound_request_protocol(self) -> Option<RequestProtocol> {
        match self {
            Self::NoConversion | Self::Unsupported => None,
            Self::OpenaiResponsesToAnthropicMessages => Some(RequestProtocol::OpenaiResponses),
            Self::AnthropicMessagesToOpenaiResponses => Some(RequestProtocol::AnthropicMessages),
        }
    }

    pub(crate) fn provider_protocol(self) -> Option<ProviderProtocol> {
        match self {
            Self::NoConversion | Self::Unsupported => None,
            Self::OpenaiResponsesToAnthropicMessages => Some(ProviderProtocol::AnthropicMessages),
            Self::AnthropicMessagesToOpenaiResponses => Some(ProviderProtocol::OpenaiResponses),
        }
    }

    pub(crate) fn module_hint(self) -> Option<&'static str> {
        match self {
            Self::NoConversion | Self::Unsupported => None,
            Self::OpenaiResponsesToAnthropicMessages => {
                Some("translation/openai_responses/to_anthropic_messages")
            }
            Self::AnthropicMessagesToOpenaiResponses => {
                Some("translation/anthropic_messages/to_openai_responses")
            }
        }
    }
}

#[cfg(test)]
#[path = "path_tests.rs"]
mod tests;
