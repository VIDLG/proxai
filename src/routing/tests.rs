use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use url::Url;

use crate::config::{
    DefaultProviderNamesConfig, MatchKind, ProviderCompatibility, ProviderConfig, RouteConfig,
};
use crate::error::InternalError;
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[test]
fn route_without_request_protocol_inherits_provider_protocol() {
    let defaults =
        EffectiveDefaultProviderNames::build(default_provider_names(), &provider_names()).unwrap();
    let routes = EffectiveRoute::build(
        &provider_protocols(),
        vec![RouteConfig {
            request_protocol: None,
            match_kind: MatchKind::Glob,
            model_pattern: "claude-*".to_string(),
            provider_name: "anthropic".to_string(),
            upstream_model: Some("claude-sonnet-4-5-20250929".to_string()),
        }],
    )
    .unwrap();

    let resolved = resolve_route(
        &defaults,
        &routes,
        RequestProtocol::OpenaiResponses,
        "claude-sonnet",
    )
    .unwrap();

    assert_eq!(resolved.provider_name, "openai");
    assert_eq!(resolved.upstream_model, "claude-sonnet");
}

#[test]
fn explicit_request_protocol_can_route_openai_ingress_to_anthropic_provider() {
    let defaults =
        EffectiveDefaultProviderNames::build(default_provider_names(), &provider_names()).unwrap();
    let routes = EffectiveRoute::build(
        &provider_protocols(),
        vec![RouteConfig {
            request_protocol: Some(RequestProtocol::OpenaiResponses),
            match_kind: MatchKind::Exact,
            model_pattern: "claude-sonnet".to_string(),
            provider_name: "anthropic".to_string(),
            upstream_model: Some("claude-sonnet-4-5-20250929".to_string()),
        }],
    )
    .unwrap();

    let resolved = resolve_route(
        &defaults,
        &routes,
        RequestProtocol::OpenaiResponses,
        "claude-sonnet",
    )
    .unwrap();

    assert_eq!(resolved.provider_name, "anthropic");
    assert_eq!(resolved.upstream_model, "claude-sonnet-4-5-20250929");
}

#[test]
fn effective_default_provider_names_reject_empty_defaults() {
    let defaults = DefaultProviderNamesConfig {
        openai_responses: "openai".to_string(),
        openai_chat_completions: "openai".to_string(),
        anthropic_messages: "   ".to_string(),
    };

    let error = EffectiveDefaultProviderNames::build(defaults, &provider_names()).unwrap_err();

    assert!(matches!(
        error,
        InternalError::InvalidProviderResolution(message)
            if message == "routing.default_provider_names.anthropic_messages must be a non-empty string"
    ));
}

#[test]
fn effective_default_provider_names_reject_unknown_defaults() {
    let defaults = DefaultProviderNamesConfig {
        openai_responses: "openai".to_string(),
        openai_chat_completions: "missing-chat".to_string(),
        anthropic_messages: "anthropic".to_string(),
    };

    let error = EffectiveDefaultProviderNames::build(defaults, &provider_names()).unwrap_err();

    assert!(matches!(
        error,
        InternalError::InvalidProviderResolution(message)
            if message == "routing.default_provider_names.openai_chat_completions references unknown provider `missing-chat`"
    ));
}

fn default_provider_names() -> DefaultProviderNamesConfig {
    DefaultProviderNamesConfig {
        openai_responses: "openai".to_string(),
        openai_chat_completions: "openai".to_string(),
        anthropic_messages: "anthropic".to_string(),
    }
}

fn provider_names() -> BTreeSet<String> {
    provider_configs()
        .into_keys()
        .map(|name| normalize_provider_name(&name))
        .collect()
}

fn provider_protocols() -> BTreeMap<String, ProviderProtocol> {
    provider_configs()
        .into_iter()
        .map(|(name, provider)| (normalize_provider_name(&name), provider.protocol))
        .collect()
}

fn provider_configs() -> BTreeMap<String, ProviderConfig> {
    BTreeMap::from([
        (
            "anthropic".to_string(),
            provider_config(ProviderProtocol::AnthropicMessages),
        ),
        (
            "openai".to_string(),
            provider_config(ProviderProtocol::OpenaiResponses),
        ),
    ])
}

fn provider_config(protocol: ProviderProtocol) -> ProviderConfig {
    ProviderConfig {
        protocol,
        base_url: Url::parse("https://example.com/").unwrap(),
        api_key: "test-key".to_string(),
        compatibility: ProviderCompatibility::default(),
        read_idle_timeout: Duration::from_secs(120),
    }
}
