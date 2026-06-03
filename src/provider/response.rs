use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::capture::CaptureSession;
use crate::config::ProviderCompatibility;

use crate::http_model::UpstreamResponseHead;
use crate::protocol::ProviderProtocol;
use crate::request::RequestId;
use crate::upstream::forward_non_streaming_response;

use super::{ProviderStreamingResponsePolicy, anthropic_messages, openai};

pub(crate) struct ProviderStreamingResponseContext<'a> {
    pub(crate) request_id: RequestId,
    pub(crate) started: std::time::Instant,
    pub(crate) capture: &'a CaptureSession,
    pub(crate) span: &'a tracing::Span,
    pub(crate) policy: ProviderStreamingResponsePolicy,
    pub(crate) compatibility: ProviderCompatibility,
}

pub(crate) struct ProviderNonStreamingResponseContext<'a> {
    pub(crate) capture: &'a CaptureSession,
    pub(crate) span: &'a tracing::Span,
    pub(crate) compatibility: ProviderCompatibility,
}

impl ProviderStreamingResponseContext<'_> {
    pub(crate) async fn handle_success_response(
        self,
        protocol: ProviderProtocol,
        response: reqwest::Response,
    ) -> Response<Body> {
        match protocol {
            ProviderProtocol::OpenaiResponses => {
                openai::responses::handle_streaming_response(self, response).await
            }
            ProviderProtocol::OpenaiChatCompletions => {
                openai::chat_completions::handle_streaming_response(self, response).await
            }
            ProviderProtocol::AnthropicMessages => {
                anthropic_messages::handle_streaming_response(self, response).await
            }
        }
    }
}

impl ProviderNonStreamingResponseContext<'_> {
    pub(crate) async fn handle_success_response(
        self,
        protocol: ProviderProtocol,
        head: UpstreamResponseHead,
        body: Bytes,
    ) -> Response<Body> {
        match protocol {
            ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
                let Self { capture, span, .. } = self;
                forward_non_streaming_response(capture, span, head, body, |body| body).await
            }
            ProviderProtocol::AnthropicMessages => {
                anthropic_messages::handle_non_streaming_response(self, head, body).await
            }
        }
    }
}
