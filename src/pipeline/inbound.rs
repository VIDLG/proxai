use axum::body::Bytes;
use axum::http::request::Parts;

use crate::config::ErrorResponseFormat;
use crate::error::{RequestError, Result};
use crate::ingress::{PreparedInboundRequest, prepare_inbound_request};
use crate::observe::{InboundRequestPrepared, ObserveContext};
use crate::protocol::RequestProtocol;

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
        obs: ObserveContext,
        error_response_format: ErrorResponseFormat,
    ) -> Self {
        Self {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            obs,
            error_response_format,
            stage: InboundHttp { body },
        }
    }

    pub(crate) fn prepare_inbound(self) -> Result<PreparedInboundFlow, RequestError> {
        let Self {
            method,
            uri,
            headers,
            obs,
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
        obs.observe_inbound_request_prepared(InboundRequestPrepared {
            method: &method,
            uri: &uri,
            headers: &headers,
            body: &body,
        });

        Ok(PreparedInboundFlow {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: PreparedInbound { request },
        })
    }
}
