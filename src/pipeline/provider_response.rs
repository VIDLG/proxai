use axum::body::Body;
use axum::http::Response;

use crate::error::{InternalError, Result};
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

        translate_streaming_response(inbound_protocol, provider_protocol, response).await
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

        translate_non_streaming_response(inbound_protocol, provider_protocol, response).await
    }
}
