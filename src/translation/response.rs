use axum::body::Body;
use axum::http::Response;

use crate::error::{InternalError, Result};
use crate::protocol::{ProviderProtocol, RequestProtocol};

pub(crate) async fn translate_response(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    response: Response<Body>,
) -> Result<Response<Body>, InternalError> {
    if !response.status().is_success() {
        return Ok(response);
    }

    match (request_protocol, provider_protocol) {
        (RequestProtocol::OpenaiResponses, ProviderProtocol::AnthropicMessages) => {
            super::anthropic_messages::to_openai_responses::translate_response(response).await
        }
        (RequestProtocol::AnthropicMessages, ProviderProtocol::OpenaiResponses) => {
            super::openai_responses::to_anthropic_messages::translate_response(response).await
        }
        _ => Ok(response),
    }
}

#[cfg(test)]
#[path = "response_tests.rs"]
mod tests;
