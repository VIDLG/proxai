use futures_util::StreamExt;
use proxai::protocol::anthropic::messages::{ContentBlock, Message, MessageStreamEvent};
use proxai::protocol::openai::chat_completions::{
    ChatResponseProjection, CreateChatCompletionResponse,
};
use proxai::protocol::openai_responses::{Response, ResponseProjection};
use proxai::protocol::{ProviderProtocol, RequestProtocol};
use proxai::{AppState, paths};
use serde_json::json;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use url::Url;

#[tokio::test]
#[ignore = "uses the real home-directory config.toml and calls the configured upstream"]
async fn live_home_config_starts_proxy_and_serves_responses_request() {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load home-directory config.toml");
    let model = std::env::var("PROXAI_LIVE_TEST_MODEL").unwrap_or_else(|_| "gpt-5.4".to_string());

    let state = AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("build app state from home-directory config.toml")
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));

    let listener = TcpListener::bind((config.server.host, 0))
        .await
        .expect("bind local proxy listener");
    let address = listener.local_addr().expect("read local proxy address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        state
            .serve(listener, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let local_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client");
    let response = local_client
        .post(format!("http://{address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer local-test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "instructions": "You are a proxai live configuration smoke test. Reply briefly.",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Reply with the exact text: proxai-live-ok"}]
            }],
            "stream": false,
            "max_output_tokens": 32
        }))
        .send()
        .await
        .expect("send request through local proxy");

    let status = response.status();
    let body = response.text().await.expect("read proxy response body");
    let _ = shutdown_tx.send(());
    server
        .await
        .expect("join local proxy task")
        .expect("stop proxy");

    assert!(
        status.is_success(),
        "expected success from live proxy request, got {status}; body: {}",
        body.chars().take(1200).collect::<String>()
    );
    assert!(!body.trim().is_empty(), "live proxy response body is empty");

    let upstream_response: async_openai::types::responses::Response = serde_json::from_str(&body)
        .unwrap_or_else(|error| {
            panic!(
                "live Responses body must deserialize as async-openai Response: {error}\nbody: {}",
                body.chars().take(1200).collect::<String>()
            )
        });
    let response = Response::from(upstream_response);
    let projection = ResponseProjection::from(&response);
    assert!(!projection.id.is_empty(), "Responses id must be non-empty");
    assert_eq!(
        projection.object, "response",
        "Responses object must be `response`"
    );
    assert!(
        !projection.output.is_empty(),
        "Responses output must not be empty"
    );
    println!(
        "PASS Responses serde/projection: id={} status={:?} output_items={}",
        projection.id,
        projection.status,
        projection.output.len()
    );
}

/// Live test: send an OpenAI Responses request through the proxy, route it to
/// an Anthropic Messages provider, and validate the translated upstream
/// response can be deserialized as OpenAI Responses for the client.
///
/// Run with:
///   cargo test live_openai_responses_to_anthropic_messages_translation_with_glm -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real routed upstream API call"]
async fn live_openai_responses_to_anthropic_messages_translation_with_glm() {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load ~/.proxai/config.toml");

    let state = AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("build app state from ~/.proxai/config.toml")
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));

    let listener = TcpListener::bind((config.server.host, 0))
        .await
        .expect("bind local proxy listener");
    let address = listener.local_addr().expect("read local proxy address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        state
            .serve(listener, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let local_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client");
    let response = local_client
        .post(format!("http://{address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer local-test-client-key")
        .timeout(Duration::from_secs(120))
        .json(&json!({
            "model": "glm-5.1",
            "instructions": "You are a proxai live translation smoke test. Reply briefly.",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "Reply with the exact text: proxai-translation-live-ok"
                }]
            }],
            "stream": false,
            "max_output_tokens": 64
        }))
        .send()
        .await
        .expect("send translated Responses request through local proxy");

    let status = response.status();
    let body = response
        .text()
        .await
        .expect("read translated response body");
    let _ = shutdown_tx.send(());
    server
        .await
        .expect("join local proxy task")
        .expect("stop proxy");

    assert!(
        status.is_success(),
        "expected success from live Responses->Anthropic translation request, got {status}; body: {}",
        body.chars().take(1200).collect::<String>()
    );
    assert!(!body.trim().is_empty(), "translated response body is empty");

    let upstream_response: async_openai::types::responses::Response = serde_json::from_str(&body)
        .unwrap_or_else(|error| {
            panic!(
                "translated live body must deserialize as OpenAI Responses: {error}\nbody: {}",
                body.chars().take(1200).collect::<String>()
            )
        });
    let response = Response::from(upstream_response);
    let projection = ResponseProjection::from(&response);
    assert!(!projection.id.is_empty(), "Responses id must be non-empty");
    assert_eq!(projection.object, "response");
    assert!(
        !projection.output.is_empty(),
        "Responses output must not be empty"
    );
    println!(
        "PASS Responses->Anthropic provider translation live: id={} status={:?} output_items={}",
        projection.id,
        projection.status,
        projection.output.len()
    );
}

/// Live test: stream an OpenAI Responses request through the proxy, route it to
/// an Anthropic Messages provider, and validate the translated stream returned
/// to the client is OpenAI Responses SSE.
///
/// Run with:
///   cargo test live_openai_responses_to_anthropic_messages_stream_translation_with_glm -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real routed upstream streaming API call"]
async fn live_openai_responses_to_anthropic_messages_stream_translation_with_glm() {
    live_openai_responses_to_anthropic_messages_stream_translation(
        "glm-5.1",
        json!([{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with the exact text: proxai-stream-translation-live-ok"
            }]
        }]),
    )
    .await;
}

/// Live test: stream a Responses request with conversation message history
/// through the Anthropic Messages provider path. This is closer to editor
/// clients such as Zed, which send prior user/assistant message items.
///
/// Run with:
///   cargo test live_openai_responses_to_anthropic_messages_stream_translation_with_glm_message_history -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real routed upstream streaming API call"]
async fn live_openai_responses_to_anthropic_messages_stream_translation_with_glm_message_history() {
    live_openai_responses_to_anthropic_messages_stream_translation(
        "glm-5.1",
        json!([
            {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "Remember this short context marker: proxai-history-marker."
                }]
            },
            {
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "I will remember proxai-history-marker.",
                    "annotations": []
                }]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "Reply with the exact text: proxai-message-history-live-ok"
                }]
            }
        ]),
    )
    .await;
}

/// Live test: same stream translation path as the GLM case, using
/// MiniMax M2.7 Highspeed from the user's real config.
///
/// Run with:
///   cargo test live_openai_responses_to_anthropic_messages_stream_translation_with_minimax -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real routed upstream streaming API call"]
async fn live_openai_responses_to_anthropic_messages_stream_translation_with_minimax() {
    live_openai_responses_to_anthropic_messages_stream_translation(
        "MiniMax-M2.7-highspeed",
        json!([{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with the exact text: proxai-stream-translation-live-ok"
            }]
        }]),
    )
    .await;
}

/// Live test matrix for the Anthropic Messages provider stream translation
/// path. Keeps the currently configured Anthropic-compatible providers in one
/// loop so regressions are easy to compare from a single run.
///
/// Run with:
///   cargo test live_openai_responses_to_anthropic_messages_stream_translation_matrix -- --ignored --nocapture
///
/// Override the default model list with:
///   PROXAI_ANTHROPIC_STREAM_MODELS=glm-5.1,mimo-v2.5
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes real routed upstream streaming API calls"]
async fn live_openai_responses_to_anthropic_messages_stream_translation_matrix() {
    let models = std::env::var("PROXAI_ANTHROPIC_STREAM_MODELS")
        .unwrap_or_else(|_| "glm-5.1,MiniMax-M2.7-highspeed,mimo-v2.5,mimo-v2.5-pro".to_string());

    for model in models
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        live_openai_responses_to_anthropic_messages_stream_translation(
            model,
            json!([{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": "Reply with the exact text: proxai-stream-translation-live-ok"
                }]
            }]),
        )
        .await;
    }
}

#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and directly calls real Anthropic-compatible upstreams"]
async fn live_anthropic_compatible_upstream_shape_diagnostic() {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load ~/.proxai/config.toml");
    let models = std::env::var("PROXAI_ANTHROPIC_COMPAT_DIAG_MODELS")
        .unwrap_or_else(|_| "glm-5.1,MiniMax-M2.7-highspeed".to_string());

    for model in models
        .split(',')
        .map(str::trim)
        .filter(|model| !model.is_empty())
    {
        let Some((provider_name, upstream_model)) =
            resolve_live_anthropic_provider_for_model(&config, model)
        else {
            println!(
                "SKIP direct Anthropic shape diagnostic: no Anthropic provider route for model={model}"
            );
            continue;
        };
        let provider = config
            .providers
            .get(&provider_name)
            .unwrap_or_else(|| panic!("provider `{provider_name}` must exist"));
        println!(
            "DIAG direct Anthropic-compatible upstream: model={model} provider={provider_name} upstream_model={upstream_model}"
        );
        diagnose_direct_anthropic_non_stream(provider, &upstream_model).await;
        diagnose_direct_anthropic_stream_tool(provider, &upstream_model).await;
    }
}

fn resolve_live_anthropic_provider_for_model(
    config: &proxai::config::AppConfig,
    model: &str,
) -> Option<(String, String)> {
    for route in &config.routing.routes {
        let provider_name = route.provider.trim().to_ascii_lowercase();
        let Some(provider) = config.providers.get(&provider_name) else {
            continue;
        };
        if provider.protocol != ProviderProtocol::AnthropicMessages {
            continue;
        }
        if matches!(
            route.request_protocol,
            Some(RequestProtocol::OpenaiChatCompletions)
        ) {
            continue;
        }
        if route.model_pattern.eq_ignore_ascii_case(model) {
            return Some((
                provider_name,
                route
                    .upstream_model
                    .clone()
                    .unwrap_or_else(|| model.to_string()),
            ));
        }
    }

    let provider_name = config
        .routing
        .default_provider_names
        .anthropic_messages
        .trim()
        .to_ascii_lowercase();
    config
        .providers
        .get(&provider_name)
        .filter(|provider| provider.protocol == ProviderProtocol::AnthropicMessages)
        .map(|_| (provider_name, model.to_string()))
}

async fn diagnose_direct_anthropic_non_stream(
    provider: &proxai::config::ProviderConfig,
    model: &str,
) {
    let response = direct_anthropic_client()
        .post(upstream_messages_url(&provider.base_url))
        .header("content-type", "application/json")
        .header("x-api-key", provider.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "Reply with exactly: proxai-direct-shape-ok"}]
        }))
        .send()
        .await
        .expect("send direct non-stream Anthropic request");

    let status = response.status();
    let body = response.text().await.expect("read direct non-stream body");
    if !status.is_success() {
        println!(
            "DIAG non-stream status={status} body={}",
            body.chars().take(800).collect::<String>()
        );
        return;
    }
    let value: serde_json::Value =
        serde_json::from_str(&body).expect("direct non-stream response must be JSON");
    println!("DIAG non-stream {}", anthropic_shape_report(&value));
}

async fn diagnose_direct_anthropic_stream_tool(
    provider: &proxai::config::ProviderConfig,
    model: &str,
) {
    let response = direct_anthropic_client()
        .post(upstream_messages_url(&provider.base_url))
        .header("content-type", "application/json")
        .header("x-api-key", provider.api_key.trim())
        .header("anthropic-version", "2023-06-01")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 64,
            "stream": true,
            "messages": [{"role": "user", "content": "Use the lookup tool once with q=proxai."}],
            "tools": [{
                "name": "lookup",
                "description": "Lookup a short value.",
                "input_schema": {
                    "type": "object",
                    "properties": {"q": {"type": "string"}},
                    "required": ["q"]
                },
                "type": "custom"
            }]
        }))
        .send()
        .await
        .expect("send direct stream Anthropic request");

    let status = response.status();
    let body = response.text().await.expect("read direct stream body");
    if !status.is_success() {
        println!(
            "DIAG stream status={status} body={}",
            body.chars().take(800).collect::<String>()
        );
        return;
    }

    let mut reports = Vec::new();
    for line in body.lines().filter_map(|line| line.strip_prefix("data: ")) {
        if line == "[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(kind) = value.get("type").and_then(|value| value.as_str()) else {
            continue;
        };
        if matches!(
            kind,
            "message_start" | "content_block_start" | "message_delta"
        ) {
            reports.push(format!("{kind}: {}", anthropic_shape_report(&value)));
        }
    }
    println!("DIAG stream {}", reports.join(" | "));
}

fn direct_anthropic_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build direct Anthropic diagnostic client")
}

fn upstream_messages_url(base_url: &Url) -> Url {
    let mut base = base_url.clone();
    base.set_query(None);
    base.set_fragment(None);
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join("v1/messages").expect("join /v1/messages")
}

fn anthropic_shape_report(value: &serde_json::Value) -> String {
    let object = value.as_object();
    let message = value
        .get("message")
        .and_then(|value| value.as_object())
        .or(object);
    let message_keys = message
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| "<none>".to_string());
    let has_message_type = message.is_some_and(|object| object.contains_key("type"));
    let mut block_reports = Vec::new();
    if let Some(content) = value
        .get("content")
        .or_else(|| value.get("content_block").map(|_| value))
        .and_then(|_| {
            value
                .get("content")
                .and_then(|value| value.as_array())
                .cloned()
                .or_else(|| value.get("content_block").cloned().map(|block| vec![block]))
        })
    {
        for block in content {
            let kind = block
                .get("type")
                .and_then(|value| value.as_str())
                .unwrap_or("<unknown>");
            block_reports.push(format!(
                "{kind}[caller={},signature={}]",
                block.get("caller").is_some(),
                block.get("signature").is_some()
            ));
        }
    }
    let server_tool_use = value
        .get("usage")
        .or_else(|| message.and_then(|object| object.get("usage")))
        .and_then(|usage| usage.get("server_tool_use"));
    let server_tool_report = server_tool_use
        .and_then(|value| value.as_object())
        .map(|object| {
            format!(
                "server_tool_use(fetch={},search={})",
                object.contains_key("web_fetch_requests"),
                object.contains_key("web_search_requests")
            )
        })
        .unwrap_or_else(|| "server_tool_use=<none>".to_string());
    format!(
        "message_keys=[{message_keys}] message_type={has_message_type} blocks=[{}] {server_tool_report}",
        block_reports.join(";")
    )
}

async fn live_openai_responses_to_anthropic_messages_stream_translation(
    model: &str,
    input: serde_json::Value,
) {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load ~/.proxai/config.toml");

    let state = AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("build app state from ~/.proxai/config.toml")
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));

    let listener = TcpListener::bind((config.server.host, 0))
        .await
        .expect("bind local proxy listener");
    let address = listener.local_addr().expect("read local proxy address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        state
            .serve(listener, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let local_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client");
    let response = local_client
        .post(format!("http://{address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer local-test-client-key")
        .timeout(Duration::from_secs(120))
        .json(&json!({
            "model": model,
            "instructions": "You are a proxai live stream translation smoke test. Reply briefly.",
            "input": input,
            "stream": true,
            "max_output_tokens": 64
        }))
        .send()
        .await
        .expect("send translated streaming Responses request through local proxy");

    let status = response.status();
    let ct = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    let mut stream_error = None;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => body.extend_from_slice(&chunk),
            Err(error) => {
                stream_error = Some(error.to_string());
                break;
            }
        }
    }
    let body = String::from_utf8_lossy(&body).to_string();
    let _ = shutdown_tx.send(());
    server
        .await
        .expect("join local proxy task")
        .expect("stop proxy");

    assert!(
        status.is_success(),
        "expected success from live Responses->Anthropic stream translation request, got {status}; body: {}",
        body.chars().take(1200).collect::<String>()
    );
    assert!(
        stream_error.is_none(),
        "translated stream ended with client-visible read error: {:?}; partial body: {}",
        stream_error,
        body.chars().take(1200).collect::<String>()
    );
    assert!(
        ct.contains("text/event-stream"),
        "translated stream content-type must be text/event-stream, got: {ct}"
    );
    assert!(
        body.contains("event: response.output_text.delta")
            || body.contains("event: response.completed"),
        "translated stream body must contain OpenAI Responses SSE events; body: {}",
        body.chars().take(1200).collect::<String>()
    );
    assert!(
        !body.contains("event: message_start"),
        "client-facing translated stream must not expose Anthropic message_start events"
    );
    println!(
        "PASS Responses->Anthropic stream translation live: model={model} bytes={} content_type={ct}",
        body.len()
    );
}

/// Live test: verify the user's MiniMax chat provider can serve MiniMax-M3
/// through the OpenAI Chat Completions-compatible proxy path.
///
/// Run with:
///   cargo test live_minimax_chat_provider_roundtrip -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and calls the configured MiniMax upstream"]
async fn live_minimax_chat_provider_roundtrip() {
    live_openai_chat_completions_roundtrip_with_model(
        "MiniMax-M3".to_string(),
        Some("minimax_chat"),
        "proxai-minimax-chat-live-ok",
    )
    .await;
}

/// Live test: roundtrip OpenAI Chat Completions through the proxy and validate
/// the response can be projected into proxai's protocol schema.
///
/// Run with:
///   PROXAI_CHAT_TEST_MODEL=gpt-5.4 cargo test live_chat -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real OpenAI-compatible API call"]
async fn live_openai_chat_completions_protocol_roundtrip() {
    let model = std::env::var("PROXAI_CHAT_TEST_MODEL")
        .or_else(|_| std::env::var("PROXAI_LIVE_TEST_MODEL"))
        .unwrap_or_else(|_| "MiniMax-M3".to_string());
    live_openai_chat_completions_roundtrip_with_model(model, None, "proxai-chat-live-ok").await;
}

async fn live_openai_chat_completions_roundtrip_with_model(
    model: String,
    expected_provider: Option<&str>,
    expected_text: &str,
) {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load ~/.proxai/config.toml");

    let provider_name = config
        .routing
        .default_provider_names
        .openai_chat_completions
        .trim()
        .to_ascii_lowercase();
    if let Some(expected_provider_name) = expected_provider {
        assert_eq!(
            provider_name, expected_provider_name,
            "expected OpenAI Chat Completions default provider to be `{expected_provider_name}`"
        );
    }
    let Some(provider) = config.providers.get(&provider_name) else {
        println!(
            "SKIP Chat Completions live: default provider `{provider_name}` is not configured"
        );
        return;
    };
    if provider.protocol != ProviderProtocol::OpenaiChatCompletions {
        println!(
            "SKIP Chat Completions live: default provider `{provider_name}` uses protocol {:?}",
            provider.protocol
        );
        return;
    }

    let state = AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("build app state from ~/.proxai/config.toml")
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));

    let listener = TcpListener::bind((config.server.host, 0))
        .await
        .expect("bind local proxy listener");
    let address = listener.local_addr().expect("read local proxy address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        state
            .serve(listener, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let local_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client");
    let response = local_client
        .post(format!("http://{address}/v1/chat/completions"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer local-test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": format!("Reply with the exact text: {expected_text}")
            }],
            "stream": false,
            "max_completion_tokens": 32
        }))
        .send()
        .await
        .expect("send Chat Completions request through local proxy");

    let status = response.status();
    let body = response.text().await.expect("read chat response body");
    let _ = shutdown_tx.send(());
    server
        .await
        .expect("join local proxy task")
        .expect("stop proxy");

    assert!(
        status.is_success(),
        "expected success from live Chat Completions request, got {status}; body: {}",
        body.chars().take(1200).collect::<String>()
    );

    let upstream_response: async_openai::types::chat::CreateChatCompletionResponse =
        serde_json::from_str(&body).unwrap_or_else(|error| {
            panic!(
                "live Chat Completions body must deserialize as async-openai response: {error}\nbody: {}",
                body.chars().take(1200).collect::<String>()
            )
        });
    let response = CreateChatCompletionResponse::from(upstream_response);
    let projection = ChatResponseProjection::from(response);
    assert!(
        !projection.id.is_empty(),
        "Chat Completions id must be non-empty"
    );
    assert_eq!(
        projection.object, "chat.completion",
        "Chat Completions object must be `chat.completion`"
    );
    assert!(
        !projection.choices.is_empty(),
        "Chat Completions choices must not be empty"
    );
    assert!(
        projection
            .choices
            .iter()
            .filter_map(|choice| choice.message.content.as_deref())
            .any(|content| content.contains(expected_text)),
        "Chat Completions response should contain `{expected_text}`; projection: {projection:?}"
    );
    println!(
        "PASS Chat Completions serde/projection: id={} model={} choices={}",
        projection.id,
        projection.model,
        projection.choices.len()
    );
}

/// Live test: roundtrip the Anthropic Messages protocol through the proxy.
/// Validates ToolUnion (16-variant), request/response types, and streaming against
/// the real Anthropic API endpoint configured in ~/.proxai/config.toml.
///
/// Run with:
///   PROXAI_ANTHROPIC_TEST_MODEL=claude-sonnet cargo test live_anthropic -- --ignored --nocapture
#[tokio::test]
#[ignore = "uses ~/.proxai/config.toml and makes a real Anthropic API call"]
async fn live_anthropic_messages_protocol_roundtrip() {
    let app_paths = paths::ensure_app_paths().expect("prepare app paths");
    let config = proxai::config::AppConfig::load(app_paths.config_path.clone())
        .expect("load ~/.proxai/config.toml");

    assert!(
        config.providers.contains_key("anthropic"),
        "~/.proxai/config.toml must have [providers.anthropic] configured"
    );

    let model = std::env::var("PROXAI_ANTHROPIC_TEST_MODEL")
        .unwrap_or_else(|_| "deepseek-v4-flash".to_string());

    let state = AppState::new(
        config.routing.default_provider_names,
        config.providers,
        config.routing.routes,
    )
    .expect("build app state from ~/.proxai/config.toml")
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));

    let listener = TcpListener::bind((config.server.host, 0))
        .await
        .expect("bind local proxy listener");
    let address = listener.local_addr().expect("read local proxy address");
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server = tokio::spawn(async move {
        state
            .serve(listener, async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    let local_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client");

    // -- Non-streaming --------------------------------------------------------
    let resp = local_client
        .post(format!("http://{address}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("authorization", "Bearer test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "Reply with exactly: anthropic-live-ok"}]
        }))
        .send()
        .await
        .expect("send non-streaming Anthropic request");

    let status = resp.status();
    let body = resp.text().await.expect("read non-streaming body");

    if !status.is_success() {
        panic!(
            "non-streaming Anthropic request failed: {status}\nbody: {}",
            body.chars().take(800).collect::<String>()
        );
    }

    eprintln!(
        "DEBUG non-streaming body (first 600 chars): {}",
        &body[..body.len().min(600)]
    );
    let msg: Message = serde_json::from_str(&body).unwrap_or_else(|e| {
        panic!(
            "response must deserialize as Message: {e}\nbody: {}",
            &body[..body.len().min(600)]
        )
    });
    assert!(!msg.id.is_empty(), "message must have non-empty id");
    // Message successfully deserialized; just validate basic nonempty invariants
    assert!(msg.usage.output_tokens > 0, "output_tokens must be > 0");
    println!(
        "PASS Non-streaming: id={}  stop_reason={:?}  content_blocks={}",
        msg.id,
        msg.stop_reason,
        msg.content.len()
    );

    // -- Streaming -----------------------------------------------------------
    let resp = local_client
        .post(format!("http://{address}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("authorization", "Bearer test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 256,
            "stream": true,
            "messages": [{"role": "user", "content": "Count from 1 to 3, one word per line."}]
        }))
        .send()
        .await
        .expect("send streaming Anthropic request");

    // status and headers are both borrows; consume resp last with .text()
    let _status = resp.status();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp.text().await.expect("read streaming body");
    assert!(
        ct.contains("text/event-stream"),
        "streaming response must be text/event-stream, got: {ct}"
    );

    let lines: Vec<&str> = body.lines().collect();
    let events: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("event: "))
        .map(|l| l.strip_prefix("event: ").unwrap())
        .collect();
    let data_lines: Vec<&str> = lines
        .iter()
        .filter(|l| l.starts_with("data: "))
        .map(|l| l.strip_prefix("data: ").unwrap())
        .collect();

    println!(
        "PASS Streaming: {} SSE events  {} data lines",
        events.len(),
        data_lines.len()
    );
    assert!(!events.is_empty(), "SSE events must not be empty");
    assert!(
        events.contains(&"message_start"),
        "must have event: message_start"
    );
    assert!(
        events.contains(&"message_delta") || events.contains(&"message_stop"),
        "must have message_delta or message_stop"
    );

    let mut event_kinds = std::collections::BTreeSet::new();
    for (i, dl) in data_lines.iter().enumerate() {
        if *dl != "[DONE]" {
            let event: MessageStreamEvent = serde_json::from_str(dl)
                .unwrap_or_else(|_| panic!("data line {i} must deserialize as MessageStreamEvent"));
            let kind = format!("{:?}", event);
            event_kinds.insert(kind.chars().take(40).collect::<String>());
        }
    }
    println!(
        "PASS All {} data lines deserialize as MessageStreamEvent",
        data_lines.len()
    );
    println!(
        "  event kinds: {:?}",
        event_kinds.iter().collect::<Vec<_>>()
    );

    // -- Custom tool (ToolUnion type="custom") -----------------------------------
    let resp = local_client
        .post(format!("http://{address}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("authorization", "Bearer test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "Use the get_weather tool for London."}],
            "tools": [{
                "name": "get_weather",
                "description": "Get current weather for a city.",
                "input_schema": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"]
                },
                "type": "custom"
            }]
        }))
        .send()
        .await
        .expect("send Anthropic request with custom tool");

    let status = resp.status();
    let body = resp.text().await.expect("read tools response body");
    if status.is_success() {
        let msg: serde_json::Value =
            serde_json::from_str(&body).expect("tools response must be valid JSON");
        let has_tool_use = msg["content"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .any(|c| c.get("type") == Some(&json!("tool_use")))
            })
            .unwrap_or(false);
        println!(
            "PASS Custom tool: status={status} has_tool_use={has_tool_use}  stop_reason={:?}",
            msg["stop_reason"]
        );
    } else {
        println!("WARN custom tool returned {status} -- upstream may not support `type: custom`");
    }

    // -- Thinking config ----------------------------------------------------
    let resp = local_client
        .post(format!("http://{address}/v1/messages"))
        .header("content-type", "application/json")
        .header("anthropic-version", "2023-06-01")
        .header("authorization", "Bearer test-client-key")
        .timeout(Duration::from_secs(90))
        .json(&json!({
            "model": model,
            "max_tokens": 512,
            "thinking": {"type": "enabled", "budget_tokens": 1024},
            "messages": [{"role": "user", "content": "What is 2+2?"}]
        }))
        .send()
        .await
        .expect("send Anthropic request with thinking config");

    let status = resp.status();
    let body = resp.text().await.expect("read thinking response body");
    if status.is_success() {
        let msg: Message =
            serde_json::from_str(&body).expect("thinking response must deserialize as Message");
        let has_thinking = msg
            .content
            .iter()
            .any(|c| matches!(c, ContentBlock::Thinking(_)));
        println!(
            "PASS Thinking: has_thinking={has_thinking}  stop_reason={:?}",
            msg.stop_reason
        );
    } else {
        println!("WARN thinking returned {status} -- model may not support extended thinking");
    }

    let _ = shutdown_tx.send(());
    server
        .await
        .expect("join local proxy task")
        .expect("stop proxy");
}
