use axum::body::Body;
use axum::http::Response;
use proxai_core::provider::{ProviderCompatibility, ProviderNormalizer};

use crate::observe::ObserveContext;
use crate::protocol::ProviderProtocol;

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

    pub(crate) fn normalizer(&self, obs: ObserveContext) -> ProviderNormalizer {
        ProviderNormalizer::new(self.protocol, self.compatibility).with_observer(obs)
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
