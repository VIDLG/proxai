use super::EffectiveRoute;
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;

use crate::config::{
    normalize_provider_name, MatchKind, ProviderCompatibility, ProviderConfig, RouteConfig,
};
use crate::error::InternalError;
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[test]
fn auto_match_kind_uses_glob_patterns_case_insensitively() {
    let route = build_route(RouteConfig {
        request_protocol: None,
        match_kind: MatchKind::Auto,
        model_pattern: "gpt-*".to_string(),
        provider_name: "openai".to_string(),
        upstream_model: Some("gpt-5.4".to_string()),
    });

    let matched = route.match_model("GPT-5.5").unwrap();

    assert_eq!(matched.as_deref(), Some("gpt-5.4"));
}

#[test]
fn auto_match_kind_uses_regex_patterns_and_supports_rewrite_templates() {
    let route = build_route(RouteConfig {
        request_protocol: None,
        match_kind: MatchKind::Auto,
        model_pattern: "^gpt-(.+)$".to_string(),
        provider_name: "openai".to_string(),
        upstream_model: Some("openai/$1".to_string()),
    });

    let matched = route.match_model("gpt-5.5").unwrap();

    assert_eq!(matched.as_deref(), Some("openai/5.5"));
}

#[test]
fn invalid_regex_patterns_return_invalid_route_errors() {
    let error = EffectiveRoute::build(
        &provider_protocols(),
        vec![RouteConfig {
            request_protocol: Some(RequestProtocol::OpenaiResponses),
            match_kind: MatchKind::Regex,
            model_pattern: "(".to_string(),
            provider_name: "openai".to_string(),
            upstream_model: None,
        }],
    )
    .unwrap_err();

    match error {
        InternalError::InvalidRoute(message) => assert!(message.contains("(")),
        other => panic!("expected invalid route error, got {other}"),
    }
}

fn build_route(route: RouteConfig) -> EffectiveRoute {
    EffectiveRoute::build(&provider_protocols(), vec![route])
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

fn provider_protocols() -> BTreeMap<String, ProviderProtocol> {
    provider_configs()
        .into_iter()
        .map(|(name, provider)| (normalize_provider_name(&name), provider.protocol))
        .collect()
}

fn provider_configs() -> BTreeMap<String, ProviderConfig> {
    BTreeMap::from([(
        "openai".to_string(),
        ProviderConfig {
            protocol: ProviderProtocol::OpenaiResponses,
            base_url: Url::parse("https://example.com/").unwrap(),
            api_key: "test-key".to_string(),
            compatibility: ProviderCompatibility::default(),
            read_idle_timeout: Duration::from_secs(120),
        },
    )])
}
