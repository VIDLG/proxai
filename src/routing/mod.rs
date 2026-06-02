mod matcher;

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{normalize_provider_name, DefaultProviderNamesConfig, RouteConfig};
use crate::error::{InternalError, Result};
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[derive(Debug, Clone)]
pub(crate) struct EffectiveDefaultProviderNames {
    pub(crate) openai_responses: String,
    pub(crate) openai_chat_completions: String,
    pub(crate) anthropic_messages: String,
}

impl EffectiveDefaultProviderNames {
    pub(crate) fn build(
        default_provider_names: DefaultProviderNamesConfig,
        provider_names: &BTreeSet<String>,
    ) -> Result<Self, InternalError> {
        let normalize_and_validate = |field_name: &str,
                                      provider: String|
         -> Result<String, InternalError> {
            let normalized = normalize_provider_name(&provider);
            if normalized.is_empty() {
                return Err(InternalError::InvalidProviderResolution(format!(
                    "routing.default_provider_names.{field_name} must be a non-empty string"
                )));
            }
            provider_names
                .contains(&normalized)
                .then_some(normalized.clone())
                    .ok_or(InternalError::InvalidProviderResolution(format!(
                        "routing.default_provider_names.{field_name} references unknown provider `{normalized}`"
                    )))
        };

        Ok(Self {
            openai_responses: normalize_and_validate(
                "openai_responses",
                default_provider_names.openai_responses,
            )?,
            openai_chat_completions: normalize_and_validate(
                "openai_chat_completions",
                default_provider_names.openai_chat_completions,
            )?,
            anthropic_messages: normalize_and_validate(
                "anthropic_messages",
                default_provider_names.anthropic_messages,
            )?,
        })
    }

    pub(crate) fn for_request_protocol(&self, request_protocol: RequestProtocol) -> &str {
        match request_protocol {
            RequestProtocol::OpenaiResponses => self.openai_responses.as_str(),
            RequestProtocol::OpenaiChatCompletions => self.openai_chat_completions.as_str(),
            RequestProtocol::AnthropicMessages => self.anthropic_messages.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EffectiveRoute {
    pub(crate) name: Option<String>,
    pub(crate) request_protocol: Option<RequestProtocol>,
    pub(crate) provider: String,
    pub(crate) upstream_model: Option<String>,
    matcher: matcher::CompiledModelMatcher,
}

impl EffectiveRoute {
    pub(crate) fn build(
        provider_protocols: &BTreeMap<String, ProviderProtocol>,
        routes: Vec<RouteConfig>,
    ) -> Result<Vec<Self>, InternalError> {
        routes
            .into_iter()
            .map(|route| {
                let provider = normalize_provider_name(&route.provider);
                let _provider_protocol = provider_protocols.get(&provider).ok_or_else(|| {
                    InternalError::InvalidProviderResolution(format!(
                        "routing.routes[].provider references unknown provider `{provider}`"
                    ))
                })?;
                let matcher =
                    matcher::CompiledModelMatcher::build(route.match_kind, &route.model_pattern)?;

                Ok(Self {
                    name: route.name,
                    request_protocol: route.request_protocol,
                    provider,
                    upstream_model: route.upstream_model,
                    matcher,
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RouteTarget {
    pub(crate) route_name: Option<String>,
    pub(crate) provider: String,
    pub(crate) upstream_model: String,
}

pub(crate) fn resolve_route(
    default_provider_names: &EffectiveDefaultProviderNames,
    routes: &[EffectiveRoute],
    request_protocol: RequestProtocol,
    model: &str,
) -> Result<RouteTarget, InternalError> {
    let mut matched = None;
    let mut protocol_mismatch = None;
    for route in routes {
        let Some(upstream_model) = route.match_model(model)? else {
            continue;
        };
        if let Some(route_protocol) = route.request_protocol {
            if route_protocol != request_protocol {
                protocol_mismatch = Some(route);
                continue;
            }
        }
        matched = Some((route, upstream_model));
        break;
    }

    if matched.is_none() {
        if let Some(route) = protocol_mismatch {
            let route_label = route
                .name
                .as_deref()
                .map(|name| format!(" `{name}`"))
                .unwrap_or_default();
            let configured = route
                .request_protocol
                .expect("protocol_mismatch is only set for explicit request_protocol");
            return Err(InternalError::InvalidRoute(format!(
                "routing route{route_label} matches model `{model}` but request_protocol is `{configured}` while the inbound request uses `{request_protocol}`; remove request_protocol to accept any inbound protocol, or update it to `{request_protocol}`"
            )));
        }
    }

    let default_provider = default_provider_names.for_request_protocol(request_protocol);
    let route_name = matched.as_ref().and_then(|(route, _)| route.name.clone());
    let provider = matched
        .as_ref()
        .map(|(route, _)| route.provider.clone())
        .unwrap_or_else(|| normalize_provider_name(default_provider));
    let upstream_model = matched
        .map(|(_, upstream_model)| upstream_model)
        .unwrap_or_else(|| model.to_string());

    Ok(RouteTarget {
        route_name,
        provider,
        upstream_model,
    })
}

#[cfg(test)]
mod tests;
