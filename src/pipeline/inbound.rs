use axum::body::Bytes;
use axum::http::request::Parts;
use serde_json::Value;

use crate::config::ErrorResponseFormat;
use crate::error::RequestError;
use crate::observe::ObserveContext;
use crate::protocol::RequestProtocol;

use super::ProxyFlow;

pub(crate) struct InboundHttp {
    body: Bytes,
}

pub(crate) struct ParsedInbound {
    pub(super) protocol: RequestProtocol,
    pub(super) payload: Value,
    pub(super) body: Bytes,
}

pub(crate) type InboundHttpFlow = ProxyFlow<InboundHttp>;
pub(crate) type ParsedInboundFlow = ProxyFlow<ParsedInbound>;

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

    pub(crate) fn parse_inbound(self) -> Result<ParsedInboundFlow, RequestError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: InboundHttp { body },
        } = self;

        let protocol = match uri.path() {
            "/v1/responses" | "/responses" => RequestProtocol::OpenaiResponses,
            "/v1/chat/completions" | "/chat/completions" => RequestProtocol::OpenaiChatCompletions,
            "/v1/messages" | "/messages" => RequestProtocol::AnthropicMessages,
            path => {
                return Err(RequestError::UnsupportedPath {
                    path: path.to_string(),
                });
            }
        };
        let payload = serde_json::from_slice::<Value>(&body)
            .map_err(|source| RequestError::InvalidJson { protocol, source })?;

        Ok(ParsedInboundFlow {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: ParsedInbound {
                protocol,
                payload,
                body,
            },
        })
    }
}

#[cfg(test)]
#[path = "inbound_tests.rs"]
mod tests;
