use std::collections::BTreeMap;

use axum::http::HeaderMap;

use crate::error::{InternalError, Result};
use crate::ingress::PreparedInboundRequest;
use crate::observe::{ProviderProtocolRequestPrepared, ProviderRequestBodySizes};
use crate::protocol::RequestProtocol;
use crate::provider::{self, ProviderRequest, ProviderTransport, ProviderTransportError};
use crate::routing::{EffectiveDefaultProviderNames, EffectiveRoute, RouteTarget, resolve_route};
use crate::translation::translate_request;

use super::ProxyFlow;
use super::inbound::{PreparedInbound, PreparedInboundFlow};
use super::upstream_response::{UpstreamHttp, UpstreamHttpFlow};

pub(crate) struct RoutedInbound {
    request: PreparedInboundRequest,
    route: RouteTarget,
    transport: ProviderTransport,
}

pub(crate) struct PreparedProvider {
    pub(super) inbound_protocol: RequestProtocol,
    transport: ProviderTransport,
    request: ProviderRequest,
}

pub(crate) type RoutedInboundFlow = ProxyFlow<RoutedInbound>;
pub(crate) type PreparedProviderFlow = ProxyFlow<PreparedProvider>;

impl PreparedInboundFlow {
    pub(crate) fn route_to_provider(
        self,
        default_provider_names: &EffectiveDefaultProviderNames,
        routes: &[EffectiveRoute],
        providers: &BTreeMap<String, ProviderTransport>,
    ) -> Result<RoutedInboundFlow, InternalError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: PreparedInbound { request },
        } = self;
        let route = resolve_route(
            default_provider_names,
            routes,
            request.protocol(),
            request.model(),
        )?;
        let transport = providers
            .get(&route.provider)
            .cloned()
            .ok_or_else(|| InternalError::InvalidProviderResolution(route.provider.clone()))?;
        Ok(RoutedInboundFlow {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: RoutedInbound {
                request,
                route,
                transport,
            },
        })
    }
}

impl RoutedInboundFlow {
    pub(crate) fn prepare_provider_request(self) -> Result<PreparedProviderFlow, InternalError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage:
                RoutedInbound {
                    request,
                    route:
                        RouteTarget {
                            provider: provider_name,
                            route_name,
                            upstream_model,
                        },
                    transport,
                },
        } = self;

        let provider_protocol = transport.protocol();
        let translated_payload = translate_request(
            request.protocol(),
            provider_protocol,
            request.normalized_payload(),
        )?;
        let provider_request = provider::prepare_request(
            provider_protocol,
            translated_payload,
            &upstream_model,
            &obs,
        )?;

        obs.observe_provider_request_prepared(ProviderProtocolRequestPrepared {
            method: method.clone(),
            uri: uri.clone(),
            request_sizes: ProviderRequestBodySizes {
                inbound: request.body_len(),
                provider: provider_request.body().len(),
            },
            request_protocol: request.protocol(),
            provider: provider_name,
            route_name: route_name.clone(),
            provider_protocol,
            provider_request: provider_request.view(),
        });

        Ok(PreparedProviderFlow {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage: PreparedProvider {
                inbound_protocol: request.protocol(),
                transport,
                request: provider_request,
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
                    inbound_protocol,
                    transport,
                    request,
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
                inbound_protocol,
                response,
                provider_response,
            },
        })
    }
}
