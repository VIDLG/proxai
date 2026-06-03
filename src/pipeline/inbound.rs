use std::time::Instant;

use axum::body::Bytes;
use axum::http::request::Parts;

use crate::capture::CaptureSession;
use crate::config::ErrorResponseFormat;
use crate::error::{RequestError, Result};
use crate::ingress::{PreparedInboundRequest, prepare_inbound_request};
use crate::protocol::RequestProtocol;
use crate::request::RequestId;

use super::ProxyFlow;

pub(crate) struct InboundHttp {
    body: Bytes,
}

pub(crate) struct PreparedInbound {
    pub(super) request: PreparedInboundRequest,
}

pub(crate) type InboundHttpFlow = ProxyFlow<InboundHttp>;
pub(crate) type PreparedInboundFlow = ProxyFlow<PreparedInbound>;

impl InboundHttpFlow {
    pub(crate) fn new(
        parts: Parts,
        body: Bytes,
        request_id: RequestId,
        started: Instant,
        span: tracing::Span,
        capture: CaptureSession,
        error_response_format: ErrorResponseFormat,
    ) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            request_id,
            started,
            span,
            capture,
            error_response_format,
            stage: InboundHttp { body },
        }
    }

    pub(crate) async fn prepare_inbound(self) -> Result<PreparedInboundFlow, RequestError> {
        let Self {
            method,
            uri,
            headers,
            request_id,
            started,
            span,
            capture,
            error_response_format,
            stage: InboundHttp { body },
        } = self;

        let request_protocol = match uri.path() {
            "/v1/responses" | "/responses" => RequestProtocol::OpenaiResponses,
            "/v1/chat/completions" | "/chat/completions" => RequestProtocol::OpenaiChatCompletions,
            "/v1/messages" | "/messages" => RequestProtocol::AnthropicMessages,
            path => {
                return Err(RequestError::Invalid(format!(
                    "unsupported request path `{path}`"
                )));
            }
        };
        let request = prepare_inbound_request(request_protocol, &body)?;
        capture
            .capture_inbound_request(&method, &uri, &headers, &body)
            .await;

        Ok(PreparedInboundFlow {
            method,
            uri,
            headers,
            request_id,
            started,
            span,
            capture,
            error_response_format,
            stage: PreparedInbound { request },
        })
    }
}
