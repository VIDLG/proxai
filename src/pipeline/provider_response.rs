use axum::body::{Body, to_bytes};
use axum::http::Response;

use crate::error::{InternalError, Result};
use crate::http_support::{into_byte_stream, json_response_from_parts, sse_response_from_parts};

use crate::observe::StreamingTranslationFailure;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::streaming::StreamTranslationFailureSink;
use crate::translation::{
    translate_non_streaming_response, translate_streaming_response_with_failure_sink,
};

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
            method,
            uri,
            obs,
            stage:
                ProviderStreamingHttp {
                    inbound_protocol,
                    provider_protocol,
                    response,
                },
            ..
        } = self;

        let failure_obs = obs.clone();
        let failure_method = method.clone();
        let failure_uri = uri.clone();
        let failure_sink = StreamTranslationFailureSink::new(move |failure| {
            failure_obs.observe_streaming_translation_failure(StreamingTranslationFailure {
                method: &failure_method,
                uri: &failure_uri,
                request_protocol: inbound_protocol,
                provider_protocol,
                failure: &failure,
            });
        });

        let (parts, body) = response.into_parts();
        let stream = translate_streaming_response_with_failure_sink(
            inbound_protocol,
            provider_protocol,
            into_byte_stream(body.into_data_stream()),
            failure_sink,
        )?;
        Ok(sse_response_from_parts(
            parts,
            obs.instrument_stream(stream),
        ))
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
            translate_non_streaming_response(inbound_protocol, provider_protocol, payload)?;
        Ok(json_response_from_parts(
            parts,
            serde_json::to_vec(&translated)?,
        ))
    }
}
