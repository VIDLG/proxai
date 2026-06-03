use crate::config::ProviderCompatibility;
use crate::error::{Result, UpstreamError};
use crate::http_model::UpstreamResponseHead;
use crate::http_utils::response_is_sse;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{
    ProviderNonStreamingResponseContext, ProviderStreamingResponseContext,
    ProviderStreamingResponsePolicy,
};
use crate::upstream::{classify_error_response, log_upstream_body_read_error};

use super::provider_response::{
    ProviderHttpFlow, ProviderNonStreamingHttp, ProviderNonStreamingHttpFlow,
    ProviderStreamingHttp, ProviderStreamingHttpFlow,
};
use super::ProxyFlow;

pub(crate) struct UpstreamHttp {
    pub(super) inbound_protocol: RequestProtocol,
    pub(super) response: reqwest::Response,
    pub(super) provider_protocol: ProviderProtocol,
    pub(super) streaming_policy: ProviderStreamingResponsePolicy,
    pub(super) compatibility: ProviderCompatibility,
}

pub(crate) type UpstreamHttpFlow = ProxyFlow<UpstreamHttp>;

impl UpstreamHttpFlow {
    pub(crate) async fn handle_upstream_response(self) -> Result<ProviderHttpFlow, UpstreamError> {
        let Self {
            method,
            uri,
            headers,
            request_id,
            started,
            span,
            capture,
            error_response_format,
            stage:
                UpstreamHttp {
                    inbound_protocol,
                    response,
                    provider_protocol,
                    streaming_policy,
                    compatibility,
                },
            ..
        } = self;

        let is_streaming = response_is_sse(&response);
        let outbound_response = if !response.status().is_success() {
            let head = UpstreamResponseHead::from_response(&response, started.elapsed());
            let body = match response.bytes().await {
                Ok(body) => body,
                Err(source) => {
                    let error = UpstreamError::ResponseBodyRead {
                        head: head.clone(),
                        source,
                    };
                    log_upstream_body_read_error(&capture, &span, &head, &error).await;
                    return Err(error);
                }
            };
            return Err(classify_error_response(&capture, &span, head, body).await);
        } else if is_streaming {
            let context = ProviderStreamingResponseContext {
                request_id,
                started,
                capture: &capture,
                span: &span,
                policy: streaming_policy,
                compatibility,
            };
            context
                .handle_success_response(provider_protocol, response)
                .await
        } else {
            let context = ProviderNonStreamingResponseContext {
                capture: &capture,
                span: &span,
                compatibility,
            };
            let head = UpstreamResponseHead::from_response(&response, started.elapsed());
            let body = match response.bytes().await {
                Ok(body) => body,
                Err(source) => {
                    let error = UpstreamError::ResponseBodyRead {
                        head: head.clone(),
                        source,
                    };
                    log_upstream_body_read_error(&capture, &span, &head, &error).await;
                    return Err(error);
                }
            };
            context
                .handle_success_response(provider_protocol, head, body)
                .await
        };

        if is_streaming {
            Ok(ProviderHttpFlow::Streaming(ProviderStreamingHttpFlow {
                method,
                uri,
                headers,
                request_id,
                started,
                span,
                capture,
                error_response_format,
                stage: ProviderStreamingHttp {
                    inbound_protocol,
                    provider_protocol,
                    response: outbound_response,
                },
            }))
        } else {
            Ok(ProviderHttpFlow::NonStreaming(
                ProviderNonStreamingHttpFlow {
                    method,
                    uri,
                    headers,
                    request_id,
                    started,
                    span,
                    capture,
                    error_response_format,
                    stage: ProviderNonStreamingHttp {
                        inbound_protocol,
                        provider_protocol,
                        response: outbound_response,
                    },
                },
            ))
        }
    }
}
