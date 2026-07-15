use axum::body::Body;
use axum::http::Response;

use crate::observe::ObserveContext;
use crate::protocol::ProviderProtocol;

use super::{ProviderStreamingResponsePolicy, anthropic_messages, openai};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProviderResponseContext {
    protocol: ProviderProtocol,
    streaming_policy: ProviderStreamingResponsePolicy,
}

impl ProviderResponseContext {
    pub(crate) fn new(
        protocol: ProviderProtocol,
        streaming_policy: ProviderStreamingResponsePolicy,
    ) -> Self {
        Self {
            protocol,
            streaming_policy,
        }
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
        ProviderProtocol::AnthropicMessages => {
            anthropic_messages::handle_streaming_response(obs, context.streaming_policy, response)
        }
    }
}
