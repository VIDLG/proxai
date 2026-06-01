use axum::http::{HeaderMap, HeaderValue};
use reqwest::Url;

use super::{is_loopback_url, ProviderRuntime};
use crate::config::ProviderCompatibility;
use crate::protocol::ProviderProtocol;

#[test]
fn identifies_loopback_upstream_urls() {
    assert!(is_loopback_url(
        &Url::parse("http://127.0.0.1:18080").unwrap()
    ));
    assert!(is_loopback_url(&Url::parse("http://[::1]:18080").unwrap()));
    assert!(is_loopback_url(
        &Url::parse("http://localhost:18080").unwrap()
    ));
}

#[test]
fn does_not_treat_remote_upstream_urls_as_loopback() {
    assert!(!is_loopback_url(
        &Url::parse("https://api.openai.com").unwrap()
    ));
    assert!(!is_loopback_url(
        &Url::parse("http://192.168.1.10:18080").unwrap()
    ));
}

#[test]
fn upstream_url_preserves_origin_base_url_paths() {
    let runtime = runtime_with_base_url("http://upstream.example:8080");

    assert_eq!(
        runtime
            .upstream_url_for_path("/v1/responses?stream=true")
            .unwrap()
            .as_str(),
        "http://upstream.example:8080/v1/responses?stream=true"
    );
}

#[test]
fn upstream_url_does_not_duplicate_matching_api_root_path() {
    let runtime = runtime_with_base_url("https://api.openai.com/v1");

    assert_eq!(
        runtime
            .upstream_url_for_path("/v1/responses")
            .unwrap()
            .as_str(),
        "https://api.openai.com/v1/responses"
    );
}

#[test]
fn upstream_url_keeps_non_matching_base_path() {
    let runtime = runtime_with_base_url("http://gateway.example/openai");

    assert_eq!(
        runtime
            .upstream_url_for_path("/v1/chat/completions")
            .unwrap()
            .as_str(),
        "http://gateway.example/openai/v1/chat/completions"
    );
}

#[test]
fn upstream_url_strips_only_exact_base_path_prefix() {
    let runtime = runtime_with_base_url("https://api.example/v1");

    assert_eq!(
        runtime
            .upstream_url_for_path("/v1/messages")
            .unwrap()
            .as_str(),
        "https://api.example/v1/messages"
    );
    assert_eq!(
        runtime
            .upstream_url_for_path("/v1/messages?stream=true")
            .unwrap()
            .as_str(),
        "https://api.example/v1/messages?stream=true"
    );
    assert_eq!(
        runtime.upstream_url_for_path("/v1").unwrap().as_str(),
        "https://api.example/v1/"
    );
    assert_eq!(
        runtime
            .upstream_url_for_path("/v1?cursor=next")
            .unwrap()
            .as_str(),
        "https://api.example/v1/?cursor=next"
    );
    assert_eq!(
        runtime
            .upstream_url_for_path("/v11/messages")
            .unwrap()
            .as_str(),
        "https://api.example/v1/v11/messages"
    );
}

#[test]
fn openai_auth_overrides_authorization_and_removes_client_x_api_key() {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_static("Bearer client"),
    );
    headers.insert(
        "x-api-key",
        HeaderValue::from_static("client-anthropic-key"),
    );
    let runtime =
        runtime_with_protocol_and_api_key(ProviderProtocol::OpenaiResponses, " upstream ");

    let headers = runtime.forwarded_request_headers(&headers, 0);

    assert_eq!(
        headers
            .get(http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok()),
        Some("Bearer upstream")
    );
    assert!(!headers.contains_key("x-api-key"));
}

#[test]
fn anthropic_auth_overrides_x_api_key_and_removes_client_authorization() {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_static("Bearer client"),
    );
    headers.insert(
        "x-api-key",
        HeaderValue::from_static("client-anthropic-key"),
    );
    let runtime =
        runtime_with_protocol_and_api_key(ProviderProtocol::AnthropicMessages, " upstream ");

    let headers = runtime.forwarded_request_headers(&headers, 0);

    assert!(!headers.contains_key(http::header::AUTHORIZATION));
    assert_eq!(
        headers
            .get("x-api-key")
            .and_then(|value| value.to_str().ok()),
        Some("upstream")
    );
}

fn runtime_with_base_url(base_url: &str) -> ProviderRuntime {
    runtime_with_base_url_and_protocol(base_url, ProviderProtocol::OpenaiResponses, "test")
}

fn runtime_with_protocol_and_api_key(protocol: ProviderProtocol, api_key: &str) -> ProviderRuntime {
    runtime_with_base_url_and_protocol("https://api.example", protocol, api_key)
}

fn runtime_with_base_url_and_protocol(
    base_url: &str,
    protocol: ProviderProtocol,
    api_key: &str,
) -> ProviderRuntime {
    let mut base_url = Url::parse(base_url).unwrap();
    if !base_url.path().ends_with('/') {
        base_url.set_path(&format!("{}/", base_url.path()));
    }
    ProviderRuntime {
        name: "test".to_string(),
        protocol,
        base_url,
        api_key: api_key.to_string(),
        compatibility: ProviderCompatibility::default(),
        client: reqwest::Client::builder().no_proxy().build().unwrap(),
    }
}
