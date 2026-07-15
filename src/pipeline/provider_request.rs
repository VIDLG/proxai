use std::collections::BTreeMap;

use axum::http::HeaderMap;
use proxai_core::pipeline::{
    Pipeline as CorePipeline, PrepareRequestError, PreparedRequest, ResponsePipeline,
};

use crate::error::{Error, InternalError, RequestError, Result};
use crate::observe::{
    InboundRequestPrepared, ProviderProtocolRequestPrepared, ProviderRequestBodySizes,
    RequestTranslationFailure,
};
use crate::provider::{self, ProviderRequest, ProviderTransport, ProviderTransportError};

use super::ProxyFlow;
use super::inbound::{ParsedInbound, ParsedInboundFlow};
use super::upstream_response::{UpstreamHttp, UpstreamHttpFlow};

pub(crate) struct PreparedProvider {
    transport: ProviderTransport,
    request: ProviderRequest,
    response_pipeline: ResponsePipeline,
}

pub(crate) type PreparedProviderFlow = ProxyFlow<PreparedProvider>;

impl ParsedInboundFlow {
    pub(crate) fn prepare_provider_request(
        self,
        pipeline: &CorePipeline,
        providers: &BTreeMap<String, ProviderTransport>,
    ) -> Result<PreparedProviderFlow> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage:
                ParsedInbound {
                    protocol,
                    payload,
                    body,
                },
        } = self;

        let prepared = match pipeline
            .clone()
            .with_observer(obs.clone())
            .prepare_request(protocol, payload)
        {
            Ok(prepared) => prepared,
            Err(PrepareRequestError::Ingress(error)) => {
                return Err(RequestError::from(error).into());
            }
            Err(PrepareRequestError::Routing(error)) => {
                return Err(InternalError::from(error).into());
            }
            Err(PrepareRequestError::Translation(failure)) => {
                let inbound = failure.inbound();
                let provider = failure.provider();
                obs.observe_request_translation_failure(RequestTranslationFailure {
                    method: &method,
                    uri: &uri,
                    normalized_payload: inbound.normalized_payload(),
                    inbound_request_bytes: inbound.normalized_payload_len(),
                    request_protocol: inbound.protocol(),
                    provider: provider.name(),
                    route_name: provider.route_name().as_deref(),
                    provider_protocol: provider.protocol(),
                    model: inbound.model(),
                    error: failure.translation_error(),
                });
                return Err(Error::from(InternalError::from(
                    (*failure).into_translation_error(),
                )));
            }
        };

        obs.observe_inbound_request_prepared(InboundRequestPrepared {
            method: &method,
            uri: &uri,
            headers: &headers,
            body: &body,
        });

        let PreparedRequest {
            inbound,
            provider,
            provider_payload,
            response,
        } = prepared;
        let transport = providers.get(provider.name()).cloned().ok_or_else(|| {
            InternalError::MissingProviderTransport {
                provider: provider.name().to_string(),
            }
        })?;
        let provider_request =
            provider::assemble_request(provider.protocol(), provider_payload, &obs)?;

        obs.observe_provider_request_prepared(ProviderProtocolRequestPrepared {
            method: method.clone(),
            uri: uri.clone(),
            request_sizes: ProviderRequestBodySizes {
                inbound: inbound.normalized_payload_len(),
                provider: provider_request.body().len(),
            },
            request_protocol: inbound.protocol(),
            provider: provider.name().to_string(),
            route_name: provider.route_name().clone(),
            provider_protocol: provider.protocol(),
            provider_request: provider_request.view(),
        });

        Ok(PreparedProviderFlow {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: PreparedProvider {
                transport,
                request: provider_request,
                response_pipeline: response,
            },
        })
    }
}

impl PreparedProviderFlow {
    pub(crate) async fn send_to_upstream(self) -> Result<UpstreamHttpFlow, ProviderTransportError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage:
                PreparedProvider {
                    transport,
                    request,
                    response_pipeline,
                },
        } = self;
        let inbound_query = uri.query().map(ToOwned::to_owned);
        let provider_response = transport.response_context();
        let response = transport
            .send(
                method.clone(),
                inbound_query,
                headers.clone(),
                request,
                &obs,
            )
            .await?;

        Ok(UpstreamHttpFlow {
            method,
            uri,
            headers: HeaderMap::new(),
            obs,
            error_response_format,
            stage: UpstreamHttp {
                response,
                provider_response,
                response_pipeline,
            },
        })
    }
}
