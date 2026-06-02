use proxai::{
    config::{MatchKind, RouteConfig},
    protocol::RequestProtocol,
};

use super::apply_route_overrides;

fn route(name: &str) -> RouteConfig {
    RouteConfig {
        name: Some(name.to_string()),
        request_protocol: Some(RequestProtocol::OpenaiResponses),
        match_kind: MatchKind::Exact,
        model_pattern: "gpt-5.5".to_string(),
        provider: "openai_default".to_string(),
        upstream_model: Some("gpt-5.5".to_string()),
    }
}

#[test]
fn applies_route_overrides_by_name() {
    let mut routes = vec![route("primary")];

    apply_route_overrides(
        &mut routes,
        &[
            "primary.request_protocol=openai_chat_completions".to_string(),
            "primary.match_kind=glob".to_string(),
            "primary.model_pattern=MiniMax-*".to_string(),
            "primary.provider=minimax_chat".to_string(),
            "primary.upstream_model=MiniMax-M3".to_string(),
        ],
    )
    .unwrap();

    assert_eq!(
        routes[0].request_protocol,
        Some(RequestProtocol::OpenaiChatCompletions)
    );
    assert_eq!(routes[0].match_kind, MatchKind::Glob);
    assert_eq!(routes[0].model_pattern, "MiniMax-*");
    assert_eq!(routes[0].provider, "minimax_chat");
    assert_eq!(routes[0].upstream_model.as_deref(), Some("MiniMax-M3"));
}

#[test]
fn empty_route_override_clears_optional_fields() {
    let mut routes = vec![route("primary")];

    apply_route_overrides(
        &mut routes,
        &[
            "primary.request_protocol=".to_string(),
            "primary.upstream_model=".to_string(),
        ],
    )
    .unwrap();

    assert_eq!(routes[0].request_protocol, None);
    assert_eq!(routes[0].upstream_model, None);
}

#[test]
fn rejects_unknown_route_override_name() {
    let mut routes = vec![route("primary")];

    let error = apply_route_overrides(
        &mut routes,
        &["missing.provider=openai_default".to_string()],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("apply --route-override"));
}
