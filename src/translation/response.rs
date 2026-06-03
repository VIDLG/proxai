use axum::body::Body;
use axum::http::Response;

use crate::error::{InternalError, Result};
use crate::http_support::NonStreamingResponse;
use crate::protocol::{ProviderProtocol, RequestProtocol};

pub(crate) fn translate_streaming_response(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    response: Response<Body>,
) -> Result<Response<Body>, InternalError> {
    if !response.status().is_success() {
        return Ok(response);
    }

    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_openai_responses::translate_streaming_response(
                response,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_streaming_response(response)
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_chat_completions::translate_streaming_response(
                response,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_streaming_response(response)
        }
        _ => Ok(response),
    }
}

pub(crate) fn translate_non_streaming_response(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    response: NonStreamingResponse,
) -> Result<Response<Body>, InternalError> {
    if !response.status.is_success() {
        return Ok(response.into_response());
    }

    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::OpenaiChatCompletions) => {
            super::openai_chat_completions::to_openai_responses::translate_non_streaming_response(
                response,
            )
        }
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_non_streaming_response(
                response,
            )
        }
        (RequestProtocol::OpenaiChatCompletions, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_chat_completions::translate_non_streaming_response(
                response,
            )
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_non_streaming_response(
                response,
            )
        }
        _ => Ok(response.into_response()),
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
