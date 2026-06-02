use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Response};
use serde_json::json;
use std::time::{Duration, Instant};

use crate::capture::CaptureSession;
use crate::config::{ErrorResponseFormat, ProviderCompatibility};
use crate::error::Result;
use crate::logging;
use crate::protocol::ProviderProtocol;
use crate::provider::{
    anthropic_messages, normalize_upstream_error_body, openai, UpstreamResponseError,
};
use crate::upstream::UpstreamResponseHead;

pub(crate) struct OutboundResponseContext<'a> {
    pub(crate) request_id: u64,
    pub(crate) started: Instant,
    pub(crate) capture: &'a CaptureSession,
    pub(crate) span: &'a tracing::Span,
    pub(crate) sse_tool_call_timeout: Option<Duration>,
    pub(crate) read_idle_timeout: Duration,
    pub(crate) error_response_format: ErrorResponseFormat,
    pub(crate) provider_protocol: ProviderProtocol,
    pub(crate) provider_compatibility: ProviderCompatibility,
}

impl OutboundResponseContext<'_> {
    pub(crate) async fn handle_response(
        self,
        upstream_response: reqwest::Response,
    ) -> Result<Response<Body>> {
        if !upstream_response.status().is_success() {
            return self.handle_error_response(upstream_response).await;
        }

        match self.provider_protocol {
            ProviderProtocol::OpenaiResponses => {
                openai::responses::handle_success_response(self, upstream_response).await
            }
            ProviderProtocol::OpenaiChatCompletions => {
                openai::chat_completions::handle_success_response(self, upstream_response).await
            }
            ProviderProtocol::AnthropicMessages => {
                anthropic_messages::handle_success_response(self, upstream_response).await
            }
        }
    }

    async fn handle_error_response(
        &self,
        upstream_response: reqwest::Response,
    ) -> Result<Response<Body>> {
        let status = upstream_response.status();
        let headers = upstream_response.headers().clone();
        let upstream_head =
            UpstreamResponseHead::from_headers(status, &headers, self.started.elapsed());
        let body = upstream_response.bytes().await?;
        let upstream_head = upstream_head.with_content_length(body.len() as u64);
        self.capture
            .capture_upstream_response_headers(&upstream_head, &headers)
            .await?;
        self.capture
            .capture_upstream_response_body(upstream_head.content_type.as_ref(), &body)
            .await?;

        let error = match normalize_upstream_error_body(&body) {
            Ok(error) => UpstreamResponseError::Protocol(error),
            Err(error) => UpstreamResponseError::from(error),
        };
        let message = error.response_message();
        let error_type = error.response_type();

        let mut response = match self.error_response_format {
            ErrorResponseFormat::Text => {
                let text = format!("upstream {}: {message}", status.as_u16());
                let mut response = Response::new(Body::from(text));
                response.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
                response
            }
            ErrorResponseFormat::Json => {
                let body = serde_json::to_vec(&json!({
                    "error": {
                        "message": message,
                        "type": error_type,
                        "status": status.as_u16()
                    }
                }))?;
                let mut response = Response::new(Body::from(body));
                response.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
        };
        *response.status_mut() = status;
        for (name, value) in &headers {
            if should_forward_error_response_header(name) {
                response.headers_mut().append(name, value.clone());
            }
        }

        self.span.in_scope(|| {
            logging::UpstreamLogRecord::HeadError {
                head: &upstream_head,
                error: &error,
            }
            .emit()
        });

        Ok(response)
    }
}

fn should_forward_error_response_header(name: &HeaderName) -> bool {
    let name = name.as_str();
    name.eq_ignore_ascii_case("retry-after")
        || name.eq_ignore_ascii_case("x-request-id")
        || name.eq_ignore_ascii_case("request-id")
        || name.starts_with("x-ratelimit-")
        || name.starts_with("anthropic-ratelimit-")
        || name.eq_ignore_ascii_case("openai-processing-ms")
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
