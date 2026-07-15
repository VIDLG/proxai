use std::collections::BTreeSet;

use proxai_core::protocol::RequestProtocol;
use proxai_core::routing::{
    DefaultProviderNames, ModelMatchKind, RouteRule, RoutingConfig, RoutingTable,
};

#[test]
fn public_routing_api_resolves_provider_label_and_upstream_model() {
    let routing = RoutingConfig {
        default_provider_names: DefaultProviderNames {
            openai_responses: "openai".to_string(),
            openai_chat_completions: "openai".to_string(),
            anthropic_messages: "anthropic".to_string(),
        },
        routes: vec![RouteRule {
            name: Some("claude".to_string()),
            request_protocol: None,
            match_kind: ModelMatchKind::Glob,
            model_pattern: "claude-*".to_string(),
            provider: "anthropic".to_string(),
            upstream_model: Some("claude-sonnet-4-5-20250929".to_string()),
        }],
    };
    let provider_names = BTreeSet::from(["anthropic".to_string(), "openai".to_string()]);
    let routing = RoutingTable::build(routing, provider_names).unwrap();

    let resolved = routing
        .resolve(RequestProtocol::OpenaiResponses, "claude-sonnet")
        .unwrap();

    assert_eq!(resolved.route_name.as_deref(), Some("claude"));
    assert_eq!(resolved.provider, "anthropic");
    assert_eq!(resolved.upstream_model, "claude-sonnet-4-5-20250929");
}
