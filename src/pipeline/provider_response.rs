use axum::body::{Body, to_bytes};
use axum::http::Response;

use crate::error::{InternalError, Result};
use crate::http_support::NonStreamingResponse;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::{translate_non_streaming_response, translate_streaming_response};

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

        translate_streaming_response(inbound_protocol, provider_protocol, response)
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
            .map_err(|error| InternalError::Io(std::io::Error::other(error.to_string())))?;
        let response = NonStreamingResponse::from_parts(parts, body);
        translate_non_streaming_response(inbound_protocol, provider_protocol, response)
    }
}
