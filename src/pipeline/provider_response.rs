use async_stream::stream;
use axum::body::{Body, to_bytes};
use axum::http::Response;
use futures_util::StreamExt;
use proxai_core::provider::ProviderNormalizer;

use crate::error::{ErrorResponseFields, InternalError, Result};
use crate::http_support::{
    ByteStream, ByteStreamError, into_byte_stream, json_response_from_parts,
    sse_response_from_parts,
};
use crate::observe::StreamingTranslationFailure;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::sse_translation::{SseTranslationStreamError, translate_sse_stream};
use crate::translation::Translator;

use super::ProxyFlow;

pub(crate) struct ProviderStreamingHttp {
    pub(super) inbound_protocol: RequestProtocol,
    pub(super) provider_protocol: ProviderProtocol,
    pub(super) normalizer: ProviderNormalizer,
    pub(super) response: Response<Body>,
}

pub(crate) struct ProviderNonStreamingHttp {
    pub(super) inbound_protocol: RequestProtocol,
    pub(super) provider_protocol: ProviderProtocol,
    pub(super) normalizer: ProviderNormalizer,
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
                    normalizer,
                    response,
                },
            ..
        } = self;

        let (parts, body) = response.into_parts();
        let input = into_byte_stream(body.into_data_stream());
        if inbound_protocol.matches_provider_protocol(provider_protocol)
            && !normalizer.requires_structured_normalization()
        {
            return Ok(sse_response_from_parts(parts, obs.instrument_stream(input)));
        }

        let translator =
            Translator::new(inbound_protocol, provider_protocol).with_observer(obs.clone());
        let mut input = translate_sse_stream(input, normalizer, translator);
        let failure_obs = obs.clone();
        let stream: ByteStream = Box::pin(stream! {
            while let Some(item) = input.next().await {
                match item {
                    Ok(chunk) => yield Ok(chunk),
                    Err(SseTranslationStreamError::Translation(failure)) => {
                        failure_obs.observe_streaming_translation_failure(
                            StreamingTranslationFailure {
                                method: &method,
                                uri: &uri,
                                request_protocol: inbound_protocol,
                                provider_protocol,
                                failure: &failure,
                            },
                        );
                        yield Ok(ErrorResponseFields::stream_translation(failure.to_string())
                            .encode_sse_event_or_fallback());
                        return;
                    }
                    Err(SseTranslationStreamError::Upstream(error)) => {
                        yield Ok(ErrorResponseFields::upstream_response_body_read(format!(
                            "upstream SSE stream error: {error}"
                        ))
                        .encode_sse_event_or_fallback());
                        return;
                    }
                    Err(error @ SseTranslationStreamError::Encoding(_)) => {
                        yield Err(Box::new(error) as ByteStreamError);
                        return;
                    }
                }
            }
        });
        Ok(sse_response_from_parts(
            parts,
            obs.instrument_stream(stream),
        ))
    }
}

impl ProviderNonStreamingHttpFlow {
    pub(crate) async fn translate_to_outbound(self) -> Result<Response<Body>, InternalError> {
        let Self {
            obs,
            stage:
                ProviderNonStreamingHttp {
                    inbound_protocol,
                    provider_protocol,
                    normalizer,
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
        let payload = normalizer.normalize_response(serde_json::from_slice(&body)?);
        let translated = Translator::new(inbound_protocol, provider_protocol)
            .with_observer(obs)
            .translate_response(payload)?;
        Ok(json_response_from_parts(
            parts,
            serde_json::to_vec(&translated)?,
        ))
    }
}
