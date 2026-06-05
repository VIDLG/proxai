use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::config::ProviderCompatibility;
use crate::http_support::UpstreamResponseHead;
use crate::observe::ObserveContext;
use crate::protocol::ProviderProtocol;
use crate::upstream::forward_non_streaming_response;

use super::{ProviderStreamingResponsePolicy, anthropic_messages, openai};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProviderResponseContext {
    protocol: ProviderProtocol,
    streaming_policy: ProviderStreamingResponsePolicy,
    compatibility: ProviderCompatibility,
}

impl ProviderResponseContext {
    pub(crate) fn new(
        protocol: ProviderProtocol,
        streaming_policy: ProviderStreamingResponsePolicy,
        compatibility: ProviderCompatibility,
    ) -> Self {
        Self {
            protocol,
            streaming_policy,
            compatibility,
        }
    }

    pub(crate) fn protocol(&self) -> ProviderProtocol {
        self.protocol
    }
}

pub(crate) fn handle_streaming_success_response(
    context: ProviderResponseContext,
    obs: &ObserveContext,
    response: reqwest::Response,
) -> Response<Body> {
    match context.protocol {
        ProviderProtocol::OpenaiResponses => {
            openai::responses::handle_streaming_response(obs, context.streaming_policy, response)
        }
        ProviderProtocol::OpenaiChatCompletions => {
            openai::chat_completions::handle_streaming_response(
                obs,
                context.streaming_policy,
                response,
            )
        }
        ProviderProtocol::AnthropicMessages => anthropic_messages::handle_streaming_response(
            obs,
            context.streaming_policy,
            context.compatibility,
            response,
        ),
    }
}

pub(crate) fn handle_non_streaming_success_response(
    context: ProviderResponseContext,
    obs: &ObserveContext,
    head: UpstreamResponseHead,
    body: Bytes,
) -> Response<Body> {
    match context.protocol {
        ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
            forward_non_streaming_response(obs, head, body, None::<fn(Bytes) -> Bytes>)
        }
        ProviderProtocol::AnthropicMessages => {
            let transform_body = matches!(
                context.compatibility,
                ProviderCompatibility::AnthropicCompatible
            )
            .then_some(|body: Bytes| {
                anthropic_messages::normalize_message_body_bytes(&body).unwrap_or(body)
            });
            forward_non_streaming_response(obs, head, body, transform_body)
        }
    }
}
