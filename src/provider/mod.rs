use axum::http::HeaderMap;

pub(crate) mod anthropic_messages;

pub(crate) mod openai;
mod request;
mod response;
mod transport;

use crate::protocol::ProviderProtocol;

pub(crate) use request::{ProviderRequest, ProviderRequestView};
pub(crate) use response::{
    handle_non_streaming_success_response, handle_streaming_success_response,
};
pub(crate) use transport::{
    ProviderStreamingResponsePolicy, ProviderTransport, ProviderTransportError,
};

pub(crate) fn apply_request_auth_headers(
    protocol: ProviderProtocol,
    headers: &mut HeaderMap,
    api_key: &str,
) {
    match protocol {
        ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
            openai::apply_request_auth_headers(headers, api_key);
        }
        ProviderProtocol::AnthropicMessages => {
            anthropic_messages::apply_request_auth_headers(headers, api_key);
        }
    }
}
