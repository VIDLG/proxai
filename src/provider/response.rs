use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::config::ProviderCompatibility;
use crate::http_support::UpstreamResponseHead;
use crate::observe::ObserveContext;
use crate::protocol::ProviderProtocol;
use crate::upstream::forward_non_streaming_response;

use super::{ProviderStreamingResponsePolicy, anthropic_messages, openai};

pub(crate) fn handle_streaming_success_response(
    protocol: ProviderProtocol,
    obs: &ObserveContext,
    policy: ProviderStreamingResponsePolicy,
    compatibility: ProviderCompatibility,
    response: reqwest::Response,
) -> Response<Body> {
    match protocol {
        ProviderProtocol::OpenaiResponses => {
            openai::responses::handle_streaming_response(obs, policy, response)
        }
        ProviderProtocol::OpenaiChatCompletions => {
            openai::chat_completions::handle_streaming_response(obs, policy, response)
        }
        ProviderProtocol::AnthropicMessages => {
            anthropic_messages::handle_streaming_response(obs, policy, compatibility, response)
        }
    }
}

pub(crate) fn handle_non_streaming_success_response(
    protocol: ProviderProtocol,
    obs: &ObserveContext,
    compatibility: ProviderCompatibility,
    head: UpstreamResponseHead,
    body: Bytes,
) -> Response<Body> {
    match protocol {
        ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
            forward_non_streaming_response(obs, head, body, |body| body)
        }
        ProviderProtocol::AnthropicMessages => {
            anthropic_messages::handle_non_streaming_response(obs, compatibility, head, body)
        }
    }
}
