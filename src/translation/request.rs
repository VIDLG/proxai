use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{TranslationResult, TranslationScope};

pub(crate) fn translate_request(
    payload: &Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let route = scope.route();
    match (route.request_protocol, route.provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => {
            Ok(payload.clone())
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_responses::to_openai_chat_completions::translate_request_payload(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::openai_responses::to_anthropic_messages::translate_request_payload(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::openai_chat_completions::to_anthropic_messages::translate_request_payload(
                payload, scope,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::anthropic_messages::to_openai_responses::translate_request_payload(
                payload, scope,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
            super::anthropic_messages::to_openai_chat_completions::translate_request_payload(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => {
            super::openai_chat_completions::to_openai_responses::translate_request_payload(
                payload, scope,
            )
        }
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
