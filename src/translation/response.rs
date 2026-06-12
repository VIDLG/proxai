use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{TranslationError, TranslationResult};
use crate::http_support::ByteStream;

pub(crate) fn translate_streaming_stream(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    input: ByteStream,
) -> TranslationResult<ByteStream> {
    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => Ok(
            super::openai_chat_completions::to_openai_responses::translate_streaming_stream(input),
        ),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            Ok(super::anthropic_messages::to_openai_responses::translate_streaming_stream(input))
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => Ok(
            super::anthropic_messages::to_openai_chat_completions::translate_streaming_stream(
                input,
            ),
        ),
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            Ok(super::openai_responses::to_anthropic_messages::translate_streaming_stream(input))
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => Ok(
            super::openai_chat_completions::to_anthropic_messages::translate_streaming_stream(
                input,
            ),
        ),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => Ok(input),
        (request_protocol, provider_protocol) => Err(TranslationError::UnsupportedResponsePair {
            from: provider_protocol,
            to: request_protocol,
        }),
    }
}

pub(crate) fn translate_non_streaming_payload(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    payload: Value,
) -> TranslationResult<Value> {
    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => Ok(payload),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_openai_responses::translate_non_streaming_payload(
                payload,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_non_streaming_payload(payload)
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_chat_completions::translate_non_streaming_payload(
                payload,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_non_streaming_payload(payload)
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_anthropic_messages::translate_non_streaming_payload(
                payload,
            )
        }
        (request_protocol, provider_protocol) => Err(TranslationError::UnsupportedResponsePair {
            from: provider_protocol,
            to: request_protocol,
        }),
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
