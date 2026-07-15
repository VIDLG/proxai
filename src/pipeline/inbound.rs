use axum::body::Bytes;
use axum::http::request::Parts;
use proxai_core::ingress::{PreparedInboundRequest, prepare_inbound_request_with_observer};
use serde_json::Value;

use crate::config::ErrorResponseFormat;
use crate::error::{RequestError, Result};
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
                return Err(RequestError::UnsupportedPath {
                    path: path.to_string(),
                });
            }
        };
        let payload =
            serde_json::from_slice::<Value>(&body).map_err(|source| RequestError::InvalidJson {
                protocol: request_protocol,
                source,
            })?;
        let request = prepare_inbound_request_with_observer(request_protocol, payload, &obs)?;
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

#[cfg(test)]
#[path = "inbound_tests.rs"]
mod tests;
