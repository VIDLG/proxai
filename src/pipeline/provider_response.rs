use async_stream::stream;
use axum::body::{Body, to_bytes};
use axum::http::Response;
use futures_util::StreamExt;
use proxai_core::pipeline::ResponsePipeline;

use crate::error::{ErrorResponseFields, InternalError, Result};
use crate::http_support::{
    ByteStream, ByteStreamError, into_byte_stream, json_response_from_parts,
    sse_response_from_parts,
};
use crate::observe::StreamingTranslationFailure;
use crate::sse_translation::{SseTranslationStreamError, translate_sse_stream};

use super::ProxyFlow;

pub(crate) struct ProviderStreamingHttp {
    pub(super) response_pipeline: ResponsePipeline,
    pub(super) response: Response<Body>,
}

pub(crate) struct ProviderNonStreamingHttp {
    pub(super) response_pipeline: ResponsePipeline,
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
                    response_pipeline,
                    response,
                },
            ..
        } = self;

        let request_protocol = response_pipeline.request_protocol();
        let provider_protocol = response_pipeline.provider_protocol();
        let (parts, body) = response.into_parts();
        let input = into_byte_stream(body.into_data_stream());
        if !response_pipeline.requires_structured_processing() {
            return Ok(sse_response_from_parts(parts, obs.instrument_stream(input)));
        }

        let mut input = translate_sse_stream(input, response_pipeline);
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
                                request_protocol,
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
            stage:
                ProviderNonStreamingHttp {
                    response_pipeline,
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
        let translated = response_pipeline.translate_response(serde_json::from_slice(&body)?)?;
        Ok(json_response_from_parts(
            parts,
            serde_json::to_vec(&translated)?,
        ))
    }
}
