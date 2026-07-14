use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::TranslationResult;
use super::streaming::StreamTranslationFailureSink;
use crate::http_support::ByteStream;

#[cfg(test)]
pub(crate) fn translate_streaming_response(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    input: ByteStream,
) -> TranslationResult<ByteStream> {
    translate_streaming_response_with_failure_sink(
        request_protocol,
        provider_protocol,
        input,
        StreamTranslationFailureSink::default(),
    )
}

/// Translate a streaming response with the pipeline's request observation sink.
/// The sink is carrier-only: pair translators still receive only protocol data.
pub(crate) fn translate_streaming_response_with_failure_sink(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    input: ByteStream,
    failure_sink: StreamTranslationFailureSink,
) -> TranslationResult<ByteStream> {
    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => Ok(
            super::openai_chat_completions::to_openai_responses::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => Ok(
            super::openai_responses::to_openai_chat_completions::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => Ok(
            super::anthropic_messages::to_openai_responses::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => Ok(
            super::anthropic_messages::to_openai_chat_completions::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => Ok(
            super::openai_responses::to_anthropic_messages::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => Ok(
            super::openai_chat_completions::to_anthropic_messages::translate_streaming_response_with_failure_sink(
                input,
                failure_sink,
            ),
        ),
        // Identity passthrough: no translation carrier exists to observe.
        _ => Ok(input),
    }
}

pub(crate) fn translate_non_streaming_response(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    payload: Value,
) -> TranslationResult<Value> {
    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiResponses)
        | (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiChatCompletions)
        | (RequestProtocol::AnthropicMessages, ProviderProtocol::AnthropicMessages) => Ok(payload),
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_openai_responses::translate_non_streaming_response(
                payload,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_non_streaming_response(
                payload,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_chat_completions::translate_non_streaming_response(
                payload,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_openai_chat_completions::translate_non_streaming_response(
                payload,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_non_streaming_response(
                payload,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_anthropic_messages::translate_non_streaming_response(
                payload,
            )
        }
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
