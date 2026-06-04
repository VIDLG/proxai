use std::collections::BTreeMap;

use axum::http::HeaderMap;
use delegate::delegate;
use getset::Getters;

use crate::error::{InternalError, Result};
use crate::ingress::PreparedInboundRequest;
use crate::observe::{ProviderProtocolRequestPrepared, ProviderRequestBodySizes};
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{ProviderRequest, ProviderTransport, ProviderTransportError};
use crate::routing::{EffectiveDefaultProviderNames, EffectiveRoute, RouteTarget, resolve_route};
use crate::translation::translate_request;

use super::ProxyFlow;
use super::inbound::{PreparedInbound, PreparedInboundFlow};
use super::upstream_response::{UpstreamHttp, UpstreamHttpFlow};

pub(crate) struct RoutedInbound {
    request: PreparedInboundRequest,
    route: RouteTarget,
    provider_protocol: ProviderProtocol,
}

#[derive(Getters)]
pub(crate) struct PreparedProvider {
    pub(super) inbound_protocol: RequestProtocol,
    #[getset(get = "pub(crate)")]
    provider_name: String,
    request: ProviderRequest,
}

pub(crate) type RoutedInboundFlow = ProxyFlow<RoutedInbound>;
pub(crate) type PreparedProviderFlow = ProxyFlow<PreparedProvider>;

impl PreparedInboundFlow {
    pub(crate) fn route_to_provider(
        self,
        default_provider_names: &EffectiveDefaultProviderNames,
        routes: &[EffectiveRoute],
        provider_protocols: &BTreeMap<String, ProviderProtocol>,
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
        let provider_protocol = provider_protocols
            .get(&route.provider)
            .copied()
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
                provider_protocol,
            },
        })
    }
}

impl RoutedInboundFlow {
    pub(crate) fn translate_to_provider(self) -> Result<PreparedProviderFlow, InternalError> {
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
                    provider_protocol,
                },
        } = self;

        let provider_request =
            translate_request(&request, provider_protocol, &upstream_model, &obs)?;

        obs.observe_provider_request_prepared(ProviderProtocolRequestPrepared {
            method: method.clone(),
            uri: uri.clone(),
            request_sizes: ProviderRequestBodySizes {
                inbound: request.body_len(),
                provider: provider_request.body().len(),
            },
            request_protocol: request.protocol(),
            provider: provider_name.clone(),
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
                provider_name,
                request: provider_request,
            },
        })
    }
}

impl PreparedProviderFlow {
    delegate! {
        to self.stage {
            pub(crate) fn provider_name(&self) -> &String;
        }
    }

    pub(crate) async fn send_to_upstream(
        self,
        transport: &ProviderTransport,
    ) -> Result<UpstreamHttpFlow, ProviderTransportError> {
        let Self {
            method,
            uri,
            headers,
            obs,
            error_response_format,
            stage:
                PreparedProvider {
                    inbound_protocol,
                    request,
                    ..
                },
        } = self;
        let inbound_query = uri.query().map(ToOwned::to_owned);
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
                provider_protocol: transport.protocol(),
                streaming_policy: transport.streaming_response_policy(),
                compatibility: transport.compatibility(),
            },
        })
    }
}
