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
                                      provider_name: String|
         -> Result<String, InternalError> {
            let normalized = normalize_provider_name(&provider_name);
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
    pub(crate) request_protocol: RequestProtocol,
    pub(crate) provider_name: String,
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
                let provider_name = normalize_provider_name(&route.provider_name);
                let provider_protocol = provider_protocols.get(&provider_name).ok_or_else(|| {
                    InternalError::InvalidProviderResolution(format!(
                        "routing.routes[].provider_name references unknown provider `{provider_name}`"
                    ))
                })?;
                let request_protocol = route
                    .request_protocol
                    .unwrap_or_else(|| provider_protocol.default_request_protocol());
                let matcher = matcher::CompiledModelMatcher::build(
                    route.match_kind,
                    &route.model_pattern,
                )?;

                Ok(Self {
                    request_protocol,
                    provider_name,
                    upstream_model: route.upstream_model,
                    matcher,
                })
            })
            .collect()
    }
}

#[derive(Clone)]
pub(crate) struct RouteTarget {
    pub(crate) provider_name: String,
    pub(crate) upstream_model: String,
}

pub(crate) fn resolve_route(
    default_provider_names: &EffectiveDefaultProviderNames,
    routes: &[EffectiveRoute],
    request_protocol: RequestProtocol,
    model: &str,
) -> Result<RouteTarget, InternalError> {
    let mut matched = None;
    for route in routes {
        if route.request_protocol != request_protocol {
            continue;
        }
        if let Some(upstream_model) = route.match_model(model)? {
            matched = Some((route, upstream_model));
            break;
        }
    }

    let default_provider_name = default_provider_names.for_request_protocol(request_protocol);
    let provider_name = matched
        .as_ref()
        .map(|(route, _)| route.provider_name.clone())
        .unwrap_or_else(|| normalize_provider_name(default_provider_name));
    let upstream_model = matched
        .map(|(_, upstream_model)| upstream_model)
        .unwrap_or_else(|| model.to_string());

    Ok(RouteTarget {
        provider_name,
        upstream_model,
    })
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
