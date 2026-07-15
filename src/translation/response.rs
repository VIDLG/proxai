use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{TranslationResult, TranslationScope};

pub(crate) fn translate_non_streaming_response(
    payload: Value,
    scope: &TranslationScope,
) -> TranslationResult<Value> {
    let route = scope.route();
    match (route.request_protocol, route.provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => Ok(payload),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_openai_responses::translate_non_streaming_response(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_non_streaming_response(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_chat_completions::translate_non_streaming_response(
                payload, scope,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_openai_chat_completions::translate_non_streaming_response(
                payload, scope,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_non_streaming_response(
                payload, scope,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_anthropic_messages::translate_non_streaming_response(
                payload, scope,
            )
        }
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
