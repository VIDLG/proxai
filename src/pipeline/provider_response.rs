use axum::body::{Body, to_bytes};
use axum::http::Response;

use crate::error::{InternalError, Result};
use crate::http_support::{into_byte_stream, json_response_from_parts, sse_response_from_parts};

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::{translate_non_streaming_payload, translate_streaming_stream};

use super::ProxyFlow;

pub(crate) struct ProviderStreamingHttp {
    pub(super) inbound_protocol: RequestProtocol,
    pub(super) provider_protocol: ProviderProtocol,
    pub(super) response: Response<Body>,
}

pub(crate) struct ProviderNonStreamingHttp {
    pub(super) inbound_protocol: RequestProtocol,
    pub(super) provider_protocol: ProviderProtocol,
    pub(super) response: Response<Body>,
}

pub(crate) enum ProviderHttpFlow {
    Streaming(ProviderStreamingHttpFlow),
    NonStreaming(ProviderNonStreamingHttpFlow),
}

pub(crate) type ProviderStreamingHttpFlow = ProxyFlow<ProviderStreamingHttp>;
pub(crate) type ProviderNonStreamingHttpFlow = ProxyFlow<ProviderNonStreamingHttp>;

impl ProviderHttpFlow {
    pub(crate) async fn translate_to_outbound(self) -> Result<Response<Body>, InternalError> {
        match self {
            Self::Streaming(flow) => flow.translate_to_outbound().await,
            Self::NonStreaming(flow) => flow.translate_to_outbound().await,
        }
    }
}

impl ProviderStreamingHttpFlow {
    pub(crate) async fn translate_to_outbound(self) -> Result<Response<Body>, InternalError> {
        let Self {
            stage:
                ProviderStreamingHttp {
                    inbound_protocol,
                    provider_protocol,
                    response,
                },
            ..
        } = self;

        let (parts, body) = response.into_parts();
        let stream = translate_streaming_stream(
            inbound_protocol,
            provider_protocol,
            into_byte_stream(body.into_data_stream()),
        )?;
        Ok(sse_response_from_parts(parts, stream))
    }
}

impl ProviderNonStreamingHttpFlow {
    pub(crate) async fn translate_to_outbound(self) -> Result<Response<Body>, InternalError> {
        let Self {
            stage:
                ProviderNonStreamingHttp {
                    inbound_protocol,
                    provider_protocol,
                    response,
                },
            ..
        } = self;

        let (parts, body) = response.into_parts();
        let body = to_bytes(body, usize::MAX)
            .await
            .map_err(InternalError::HttpBodyRead)?;
        if !parts.status.is_success() {
            return Ok(Response::from_parts(parts, Body::from(body)));
        }
        let payload = serde_json::from_slice(&body)?;
        let translated =
            translate_non_streaming_payload(inbound_protocol, provider_protocol, payload)?;
        Ok(json_response_from_parts(
            parts,
            serde_json::to_vec(&translated)?,
        ))
    }
}
