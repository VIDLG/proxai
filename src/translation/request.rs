use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::TranslationResult;

pub(crate) fn translate_request(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    payload: &Value,
) -> TranslationResult<Value> {
    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => {
            Ok(payload.clone())
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_responses::to_openai_chat_completions::translate_request_payload(payload)
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::openai_responses::to_anthropic_messages::translate_request_payload(payload)
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::openai_chat_completions::to_anthropic_messages::translate_request_payload(
                payload,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::anthropic_messages::to_openai_responses::translate_request_payload(payload)
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
            super::anthropic_messages::to_openai_chat_completions::translate_request_payload(
                payload,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => {
            super::openai_chat_completions::to_openai_responses::translate_request_payload(payload)
        }
    }
}

#[cfg(test)]
#[path = "request_tests.rs"]
mod tests;
