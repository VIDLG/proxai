use super::*;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::AppState;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn loads_config_file_with_providers_and_routes() {
    let path = unique_config_path();
    fs::write(
        &path,
        r#"
[server]
host = "0.0.0.0"
port = 19090

[mcp]
host = "127.0.0.1"
port = 19091

[routing.default_provider_names]
openai_responses = "openai_default"
openai_chat_completions = "openai_default"
anthropic_messages = "anthropic_default"

[[routing.routes]]
match_kind = "glob"
model_pattern = "gpt-*"
provider_name = "openai_default"
upstream_model = "gpt-5.4"

[[routing.routes]]
match_kind = "exact"
request_protocol = "openai_responses"
model_pattern = "claude-sonnet"
provider_name = "anthropic_default"
upstream_model = "claude-sonnet-4-5-20250929"

[providers.openai_default]
protocol = "openai_responses"
base_url = "http://upstream.example:8080"
api_key = "replace-with-your-api-key"
read_idle_timeout_secs = 42

[providers.anthropic_default]
protocol = "anthropic_messages"
base_url = "https://api.anthropic.com"
api_key = "anthropic-secret"
read_idle_timeout_secs = 55

[tool_calls]
timeout_secs = 120

[logging]
level = "debug"
output_format = "json"
use_color = false

[logging.duration_thresholds]
warn_ms = 2500
error_ms = 9000

[error_responses]
format = "json"

[capture]
inbound_request_enabled = true
forwarded_request_enabled = false
upstream_response_enabled = false
outbound_response_enabled = false
"#,
    )
    .unwrap();

    let config = AppConfig::load(path.clone()).unwrap();
    assert_eq!(config.server.host, "0.0.0.0".parse::<IpAddr>().unwrap());
    assert_eq!(config.server.port, 19090);
    assert_eq!(config.mcp.host, "127.0.0.1".parse::<IpAddr>().unwrap());
    assert_eq!(config.mcp.port, 19091);
    assert_eq!(
        config.routing.default_provider_names.openai_responses,
        "openai_default"
    );
    assert_eq!(config.routing.routes.len(), 2);
    assert_eq!(config.routing.routes[0].request_protocol, None);
    assert_eq!(
        config.routing.routes[1].request_protocol,
        Some(RequestProtocol::OpenaiResponses)
    );
    assert_eq!(config.routing.routes[1].model_pattern, "claude-sonnet");
    assert_eq!(
        config.providers["openai_default"].base_url.as_str(),
        "http://upstream.example:8080/"
    );
    assert_eq!(
        config.providers["openai_default"].api_key,
        "replace-with-your-api-key"
    );
    assert_eq!(
        config.providers["openai_default"].read_idle_timeout,
        Duration::from_secs(42)
    );
    assert_eq!(
        config.providers["anthropic_default"].protocol,
        ProviderProtocol::AnthropicMessages
    );
    assert_eq!(config.logging.level, LogLevel::Debug);
    assert_eq!(config.logging.output_format, LogOutputFormat::Json);
    assert!(!config.logging.use_color);
    assert_eq!(config.logging.duration_thresholds.warn_ms, 2500);
    assert_eq!(config.logging.duration_thresholds.error_ms, 9000);
    assert_eq!(config.error_responses.format, ErrorResponseFormat::Json);
    assert_eq!(config.tool_calls.timeout, Duration::from_secs(120));
    assert!(config.capture.inbound_request_enabled);
    assert!(!config.capture.forwarded_request_enabled);
    assert!(!config.capture.upstream_response_enabled);
    assert!(!config.capture.outbound_response_enabled);

    fs::remove_file(path).unwrap();
}

#[test]
fn rejects_empty_provider_api_key() {
    let path = unique_config_path();
    fs::write(
        &path,
        r#"
[providers.openai_default]
protocol = "openai_responses"
base_url = "http://upstream.example:8080"
api_key = ""
read_idle_timeout_secs = 42
"#,
    )
    .unwrap();

    let error = AppConfig::load(path.clone()).unwrap_err().to_string();

    assert!(error.contains("providers.openai_default.api_key must be a non-empty string"));

    fs::remove_file(path).unwrap();
}

#[test]
fn bundled_example_config_builds_runtime_for_all_default_protocols() {
    let path = unique_config_path();
    fs::write(&path, include_str!("../config.example.toml")).unwrap();

    let config = AppConfig::load(path.clone()).unwrap();
    assert_eq!(
        config
            .providers
            .get(&config.routing.default_provider_names.openai_responses)
            .map(|provider| provider.protocol),
        Some(ProviderProtocol::OpenaiResponses)
    );
    assert_eq!(
        config
            .providers
            .get(
                &config
                    .routing
                    .default_provider_names
                    .openai_chat_completions,
            )
            .map(|provider| provider.protocol),
        Some(ProviderProtocol::OpenaiChatCompletions)
    );
    assert_eq!(
        config
            .providers
            .get(&config.routing.default_provider_names.anthropic_messages)
            .map(|provider| provider.protocol),
        Some(ProviderProtocol::AnthropicMessages)
    );

    AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("bundled example config should build runtime state");

    fs::remove_file(path).unwrap();
}

#[test]
fn parse_errors_include_config_path_and_recovery_hint() {
    let path = unique_config_path();
    fs::write(
        &path,
        r#"
[logging]
duration_green_ms = 5000
"#,
    )
    .unwrap();

    let error = AppConfig::load(path.clone()).unwrap_err().to_string();

    assert!(error.contains(&path.display().to_string()));
    assert!(error.contains("config.example.toml"));
    assert!(error.contains("regenerate defaults"));

    fs::remove_file(path).unwrap();
}

fn unique_config_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("proxai-config-{}-{nanos}.toml", std::process::id()))
}
