mod error;
mod matcher;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::protocol::RequestProtocol;

pub use error::{Result, RoutingError};
use matcher::CompiledModelMatcher;

/// Structured routing configuration accepted by the carrier-independent routing table.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoutingConfig {
    pub default_provider_names: DefaultProviderNames,
    pub routes: Vec<RouteRule>,
}

impl RoutingConfig {
    /// Validate route invariants that do not depend on the provider registry.
    pub fn validate(&self) -> Result<()> {
        let mut route_names = BTreeSet::new();
        for (index, route) in self.routes.iter().enumerate() {
            if let Some(name) = route.name.as_deref() {
                let name = name.trim();
                if name.is_empty() {
                    return Err(RoutingError::EmptyRouteName { index });
                }
                if !route_names.insert(name) {
                    return Err(RoutingError::DuplicateRouteName {
                        index,
                        name: name.to_string(),
                    });
                }
            }
            if route.model_pattern.trim().is_empty() {
                return Err(RoutingError::EmptyModelPattern { index });
            }
            if normalize_provider_name(&route.provider).is_empty() {
                return Err(RoutingError::EmptyRouteProvider { index });
            }
        }
        Ok(())
    }
}

/// Default provider labels keyed by the actual inbound request protocol.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultProviderNames {
    pub openai_responses: String,
    pub openai_chat_completions: String,
    pub anthropic_messages: String,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum ModelMatchKind {
    /// Infer glob or regex syntax from the pattern, otherwise use exact matching.
    #[default]
    Auto,
    /// Match the model name case-insensitively as a complete string.
    Exact,
    /// Match the model name against a case-insensitive glob.
    Glob,
    /// Match the model name against a case-insensitive regular expression.
    Regex,
}

/// One ordered model-routing rule.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RouteRule {
    pub name: Option<String>,
    pub request_protocol: Option<RequestProtocol>,
    pub match_kind: ModelMatchKind,
    pub model_pattern: String,
    pub provider: String,
    pub upstream_model: Option<String>,
}

/// Compiled, validated protocol/model router.
#[derive(Debug, Clone)]
pub struct RoutingTable {
    default_provider_names: DefaultProviderNames,
    routes: Vec<CompiledRoute>,
}

impl RoutingTable {
    /// Validate and compile structured routing rules against available provider labels.
    pub fn build<I, S>(config: RoutingConfig, provider_names: I) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        config.validate()?;
        let mut normalized_provider_names = BTreeSet::new();
        for name in provider_names {
            let provider = normalize_provider_name(name.as_ref());
            if provider.is_empty() {
                return Err(RoutingError::EmptyProviderName);
            }
            if !normalized_provider_names.insert(provider.clone()) {
                return Err(RoutingError::DuplicateProviderName { provider });
            }
        }
        let provider_names = normalized_provider_names;
        let default_provider_names = config
            .default_provider_names
            .normalize_and_validate(&provider_names)?;
        let routes = config
            .routes
            .into_iter()
            .enumerate()
            .map(|(index, route)| CompiledRoute::build(index, route, &provider_names))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            default_provider_names,
            routes,
        })
    }

    /// Resolve the first matching route, or the default provider for the inbound protocol.
    pub fn resolve(&self, request_protocol: RequestProtocol, model: &str) -> Result<ResolvedRoute> {
        let mut protocol_mismatch = None;
        for route in &self.routes {
            let Some(upstream_model) = route.match_model(model) else {
                continue;
            };
            if let Some(configured) = route.request_protocol
                && configured != request_protocol
            {
                protocol_mismatch.get_or_insert(route);
                continue;
            }
            return Ok(ResolvedRoute {
                route_name: route.name.clone(),
                provider: route.provider.clone(),
                upstream_model,
            });
        }

        if let Some(route) = protocol_mismatch {
            let configured = route
                .request_protocol
                .expect("protocol mismatch requires an explicit request protocol");
            return Err(RoutingError::RequestProtocolMismatch {
                route_name: route.name.clone(),
                model: model.to_string(),
                configured,
                inbound: request_protocol,
            });
        }

        Ok(ResolvedRoute {
            route_name: None,
            provider: self
                .default_provider_names
                .for_request_protocol(request_protocol)
                .to_string(),
            upstream_model: model.to_string(),
        })
    }
}

/// Provider/model selection produced by [`RoutingTable::resolve`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedRoute {
    pub route_name: Option<String>,
    pub provider: String,
    pub upstream_model: String,
}

impl DefaultProviderNames {
    fn normalize_and_validate(self, provider_names: &BTreeSet<String>) -> Result<Self> {
        Ok(Self {
            openai_responses: validate_default_provider(
                RequestProtocol::OpenaiResponses,
                self.openai_responses,
                provider_names,
            )?,
            openai_chat_completions: validate_default_provider(
                RequestProtocol::OpenaiChatCompletions,
                self.openai_chat_completions,
                provider_names,
            )?,
            anthropic_messages: validate_default_provider(
                RequestProtocol::AnthropicMessages,
                self.anthropic_messages,
                provider_names,
            )?,
        })
    }

    fn for_request_protocol(&self, request_protocol: RequestProtocol) -> &str {
        match request_protocol {
            RequestProtocol::OpenaiResponses => &self.openai_responses,
            RequestProtocol::OpenaiChatCompletions => &self.openai_chat_completions,
            RequestProtocol::AnthropicMessages => &self.anthropic_messages,
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledRoute {
    name: Option<String>,
    request_protocol: Option<RequestProtocol>,
    provider: String,
    upstream_model: Option<String>,
    matcher: CompiledModelMatcher,
}

impl CompiledRoute {
    fn build(index: usize, route: RouteRule, provider_names: &BTreeSet<String>) -> Result<Self> {
        let provider = normalize_provider_name(&route.provider);
        if !provider_names.contains(&provider) {
            return Err(RoutingError::UnknownRouteProvider { index, provider });
        }
        let matcher = CompiledModelMatcher::build(index, route.match_kind, &route.model_pattern)?;
        Ok(Self {
            name: route.name.map(|name| name.trim().to_string()),
            request_protocol: route.request_protocol,
            provider,
            upstream_model: route.upstream_model,
            matcher,
        })
    }
}

fn validate_default_provider(
    protocol: RequestProtocol,
    provider: String,
    provider_names: &BTreeSet<String>,
) -> Result<String> {
    let provider = normalize_provider_name(&provider);
    if provider.is_empty() {
        return Err(RoutingError::EmptyDefaultProvider { protocol });
    }
    if !provider_names.contains(&provider) {
        return Err(RoutingError::UnknownDefaultProvider { protocol, provider });
    }
    Ok(provider)
}

/// Normalize a user-defined provider label for lookup and routing.
pub fn normalize_provider_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests;
