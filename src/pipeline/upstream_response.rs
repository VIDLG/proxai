use crate::error::{Result, UpstreamError, UpstreamResponseError};
use crate::http_support::UpstreamResponseHead;
use crate::http_support::response_is_sse;
use crate::observe::UpstreamErrorResponseReceived;

use crate::provider::{ProviderResponseContext, handle_streaming_success_response};
use crate::upstream::forward_non_streaming_response;
use proxai_core::pipeline::ResponsePipeline;

use super::ProxyFlow;
use super::provider_response::{
    ProviderHttpFlow, ProviderNonStreamingHttp, ProviderNonStreamingHttpFlow,
    ProviderStreamingHttp, ProviderStreamingHttpFlow,
};

pub(crate) struct UpstreamHttp {
    pub(super) response: reqwest::Response,
    pub(super) provider_response: ProviderResponseContext,
    pub(super) response_pipeline: ResponsePipeline,
}

pub(crate) type UpstreamHttpFlow = ProxyFlow<UpstreamHttp>;

impl UpstreamHttpFlow {
    pub(crate) async fn handle_upstream_response(self) -> Result<ProviderHttpFlow, UpstreamError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage:
                UpstreamHttp {
                    response,
                    provider_response,
                    response_pipeline,
                },
            ..
        } = self;

        let provider_protocol = response_pipeline.provider_protocol();
        if !response.status().is_success() {
            let head = UpstreamResponseHead::from_response(&response, obs.elapsed());
            let body = match response.bytes().await {
                Ok(body) => body,
                Err(source) => {
                    let error = UpstreamError::ResponseBodyRead {
                        head: head.clone(),
                        source,
                    };
                    obs.observe_upstream_body_read_error(&head, &error);
                    return Err(error);
                }
            };
            let parsed = UpstreamResponseError::parse_body(provider_protocol, &body);
            let error = UpstreamError::ErrorStatus {
                head: head.clone(),
                body,
                parsed,
            };
            if let UpstreamError::ErrorStatus { body, .. } = &error {
                obs.observe_upstream_error_response(
                    UpstreamErrorResponseReceived { head: &head, body },
                    &error,
                );
            }
            return Err(error);
        }

        if response_is_sse(&response) {
            let response = handle_streaming_success_response(provider_response, &obs, response);
            return Ok(ProviderHttpFlow::Streaming(ProviderStreamingHttpFlow {
                method,
                uri,
                headers,
                obs,
                error_response_format,
                stage: ProviderStreamingHttp {
                    response_pipeline,
                    response,
                },
            }));
        }

        let head = UpstreamResponseHead::from_response(&response, obs.elapsed());
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(source) => {
                let error = UpstreamError::ResponseBodyRead {
                    head: head.clone(),
                    source,
                };
                obs.observe_upstream_body_read_error(&head, &error);
                return Err(error);
            }
        };
        let response = forward_non_streaming_response(&obs, head, body);
        Ok(ProviderHttpFlow::NonStreaming(
            ProviderNonStreamingHttpFlow {
                method,
                uri,
                headers,
                obs,
                error_response_format,
                stage: ProviderNonStreamingHttp {
                    response_pipeline,
                    response,
                },
            },
        ))
    }
}
