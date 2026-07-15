use std::collections::BTreeSet;

use super::*;

#[test]
fn route_without_request_protocol_accepts_actual_inbound_protocol() {
    let router = router(vec![RouteRule {
        name: None,
        request_protocol: None,
        match_kind: ModelMatchKind::Glob,
        model_pattern: "claude-*".to_string(),
        provider: "anthropic".to_string(),
        upstream_model: Some("claude-sonnet-4-5-20250929".to_string()),
    }]);

    let resolved = router
        .resolve(RequestProtocol::OpenaiResponses, "claude-sonnet")
        .unwrap();

    assert_eq!(resolved.route_name, None);
    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.upstream_model, "claude-sonnet-4-5-20250929");
}

#[test]
fn explicit_request_protocol_can_route_to_a_cross_protocol_provider() {
    let router = router(vec![RouteRule {
        name: Some("claude_responses_ant".to_string()),
        request_protocol: Some(RequestProtocol::OpenaiResponses),
        match_kind: ModelMatchKind::Exact,
        model_pattern: "claude-sonnet".to_string(),
        provider: "anthropic".to_string(),
        upstream_model: Some("claude-sonnet-4-5-20250929".to_string()),
    }]);

    let resolved = router
        .resolve(RequestProtocol::OpenaiResponses, "claude-sonnet")
        .unwrap();

    assert_eq!(resolved.route_name.as_deref(), Some("claude_responses_ant"));
    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.upstream_model, "claude-sonnet-4-5-20250929");
}

#[test]
fn explicit_request_protocol_mismatch_is_reported_for_matching_model() {
    let router = router(vec![RouteRule {
        name: Some("glm_responses_ant".to_string()),
        request_protocol: Some(RequestProtocol::OpenaiResponses),
        match_kind: ModelMatchKind::Glob,
        model_pattern: "glm-*".to_string(),
        provider: "anthropic".to_string(),
        upstream_model: None,
    }]);

    let error = router
        .resolve(RequestProtocol::OpenaiChatCompletions, "glm-5.1")
        .unwrap_err();

    assert!(matches!(
        error,
        RoutingError::RequestProtocolMismatch {
            route_name: Some(name),
            model,
            configured: RequestProtocol::OpenaiResponses,
            inbound: RequestProtocol::OpenaiChatCompletions,
        } if name == "glm_responses_ant" && model == "glm-5.1"
    ));
}

#[test]
fn later_protocol_compatible_route_wins_over_an_earlier_mismatch() {
    let router = router(vec![
        RouteRule {
            name: Some("glm_responses".to_string()),
            request_protocol: Some(RequestProtocol::OpenaiResponses),
            match_kind: ModelMatchKind::Glob,
            model_pattern: "glm-*".to_string(),
            provider: "openai".to_string(),
            upstream_model: None,
        },
        RouteRule {
            name: Some("glm_chat".to_string()),
            request_protocol: Some(RequestProtocol::OpenaiChatCompletions),
            match_kind: ModelMatchKind::Glob,
            model_pattern: "glm-*".to_string(),
            provider: "anthropic".to_string(),
            upstream_model: None,
        },
    ]);

    let resolved = router
        .resolve(RequestProtocol::OpenaiChatCompletions, "glm-5.1")
        .unwrap();

    assert_eq!(resolved.route_name.as_deref(), Some("glm_chat"));
    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.upstream_model, "glm-5.1");
}

#[test]
fn defaults_are_selected_by_actual_inbound_protocol() {
    let router = router(Vec::new());

    let resolved = router
        .resolve(RequestProtocol::AnthropicMessages, "claude-sonnet")
        .unwrap();

    assert_eq!(resolved.route_name, None);
    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.upstream_model, "claude-sonnet");
}

#[test]
fn auto_match_kind_uses_globs_case_insensitively() {
    let router = router(vec![RouteRule {
        name: None,
        request_protocol: None,
        match_kind: ModelMatchKind::Auto,
        model_pattern: "gpt-*".to_string(),
        provider: "openai".to_string(),
        upstream_model: Some("gpt-5.4".to_string()),
    }]);

    let resolved = router
        .resolve(RequestProtocol::OpenaiResponses, "GPT-5.5")
        .unwrap();

    assert_eq!(resolved.upstream_model, "gpt-5.4");
}

#[test]
fn auto_match_kind_uses_regex_rewrite_templates() {
    let router = router(vec![RouteRule {
        name: None,
        request_protocol: None,
        match_kind: ModelMatchKind::Auto,
        model_pattern: "^gpt-(.*)$".to_string(),
        provider: "openai".to_string(),
        upstream_model: Some("openai/$1".to_string()),
    }]);

    let resolved = router
        .resolve(RequestProtocol::OpenaiResponses, "gpt-5.5")
        .unwrap();

    assert_eq!(resolved.upstream_model, "openai/5.5");
}

#[test]
fn invalid_regex_preserves_the_structured_source_error() {
    let error = RoutingTable::build(
        routing_config(vec![RouteRule {
            name: None,
            request_protocol: None,
            match_kind: ModelMatchKind::Regex,
            model_pattern: "(".to_string(),
            provider: "openai".to_string(),
            upstream_model: None,
        }]),
        provider_names(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        RoutingError::InvalidRegex {
            index: 0,
            pattern,
            ..
        } if pattern == "("
    ));
}

#[test]
fn routing_config_rejects_duplicate_trimmed_route_names() {
    let config = routing_config(vec![
        RouteRule {
            name: Some("primary".to_string()),
            model_pattern: "gpt-*".to_string(),
            provider: "openai".to_string(),
            ..RouteRule::default()
        },
        RouteRule {
            name: Some(" primary ".to_string()),
            model_pattern: "claude-*".to_string(),
            provider: "anthropic".to_string(),
            ..RouteRule::default()
        },
    ]);

    assert!(matches!(
        config.validate(),
        Err(RoutingError::DuplicateRouteName { index: 1, name }) if name == "primary"
    ));
}

#[test]
fn router_rejects_empty_and_unknown_default_provider_names() {
    let mut config = routing_config(Vec::new());
    config.default_provider_names.anthropic_messages = "   ".to_string();
    assert!(matches!(
        RoutingTable::build(config, provider_names()),
        Err(RoutingError::EmptyDefaultProvider {
            protocol: RequestProtocol::AnthropicMessages
        })
    ));

    let mut config = routing_config(Vec::new());
    config.default_provider_names.openai_chat_completions = "missing-chat".to_string();
    assert!(matches!(
        RoutingTable::build(config, provider_names()),
        Err(RoutingError::UnknownDefaultProvider {
            protocol: RequestProtocol::OpenaiChatCompletions,
            provider,
        }) if provider == "missing-chat"
    ));
}

#[test]
fn router_rejects_empty_and_colliding_provider_registry_names() {
    assert!(matches!(
        RoutingTable::build(routing_config(Vec::new()), ["openai", "   ", "anthropic"]),
        Err(RoutingError::EmptyProviderName)
    ));

    assert!(matches!(
        RoutingTable::build(
            routing_config(Vec::new()),
            ["openai", " OpenAI ", "anthropic"],
        ),
        Err(RoutingError::DuplicateProviderName { provider }) if provider == "openai"
    ));
}

fn router(routes: Vec<RouteRule>) -> RoutingTable {
    RoutingTable::build(routing_config(routes), provider_names()).unwrap()
}

fn routing_config(routes: Vec<RouteRule>) -> RoutingConfig {
    RoutingConfig {
        default_provider_names: DefaultProviderNames {
            openai_responses: "openai".to_string(),
            openai_chat_completions: "openai".to_string(),
            anthropic_messages: "anthropic".to_string(),
        },
        routes,
    }
}

fn provider_names() -> BTreeSet<String> {
    ["anthropic".to_string(), "openai".to_string()]
        .into_iter()
        .collect()
}
