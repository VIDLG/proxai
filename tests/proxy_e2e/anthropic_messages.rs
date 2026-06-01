use super::common::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn proxy_forwards_anthropic_messages_without_conversion() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .header("authorization", "Bearer client-key")
        .json(&json!({
            "model": "claude-sonnet",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        }))
        .send()
        .await
        .unwrap();

    let status = response.status();
    if status != StatusCode::OK {
        panic!(
            "expected 200, got {status}; body: {}",
            response.text().await.unwrap()
        );
    }
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"ok": true}));

    let payloads = capture.payloads.lock().await;
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["model"], "claude-sonnet");
    assert_eq!(payloads[0]["messages"].as_array().unwrap().len(), 1);

    let authorizations = capture.authorizations.lock().await;
    assert_eq!(authorizations.as_slice(), &[None]);

    let api_keys = capture.api_keys.lock().await;
    assert_eq!(
        api_keys.as_slice(),
        &[Some("test-upstream-key".to_string())]
    );

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/messages".to_string()]);
}

#[tokio::test]
async fn proxy_normalizes_anthropic_messages_compatible_response() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_anthropic_messages_compat_upstream(capture.clone()).await;
    let shim_address = spawn_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "glm-5.1",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(body["type"], "message");
    assert_eq!(body["content"][0]["caller"]["type"], "direct");
    assert_eq!(
        body["usage"]["server_tool_use"]["web_fetch_requests"],
        json!(0)
    );
    assert_eq!(
        body["usage"]["server_tool_use"]["web_search_requests"],
        json!(1)
    );
}

#[tokio::test]
async fn proxy_leaves_anthropic_messages_strict_response_unchanged() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_anthropic_messages_compat_upstream(capture.clone()).await;
    let shim_address = spawn_anthropic_strict_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "glm-5.1",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(body.get("type"), None);
    assert_eq!(body["content"][0].get("caller"), None);
    assert_eq!(
        body["usage"]["server_tool_use"].get("web_fetch_requests"),
        None
    );
}

#[tokio::test]
async fn proxy_rewrites_anthropic_messages_model_from_route() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_anthropic_shim_with_model_route(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "claude-request",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let payloads = capture.payloads.lock().await;
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["model"], "claude-upstream");
}

#[tokio::test]
async fn proxy_preserves_useful_upstream_error_headers_for_anthropic_messages() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_error_upstream(capture.clone()).await;
    let shim_address = spawn_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "claude-sonnet",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": false
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok()),
        Some("7")
    );
    assert_eq!(
        response
            .headers()
            .get("request-id")
            .and_then(|value| value.to_str().ok()),
        Some("anthropic_req_123")
    );
    assert_eq!(
        response
            .headers()
            .get("anthropic-ratelimit-requests-remaining")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("upstream 429: quota exhausted"));

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/messages".to_string()]);
}

#[tokio::test]
async fn proxy_normalizes_anthropic_messages_sse_response() {
    let upstream_address = spawn_anthropic_messages_compat_sse_upstream().await;
    let shim_address = spawn_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "claude-sonnet",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: message_stop"));
    assert!(body.contains("\"message\":{\"content\":[]"));
    assert!(body.contains("\"caller\":{\"type\":\"direct\"}"));
    assert!(body.contains("\"web_fetch_requests\":0"));
}

#[tokio::test]
async fn proxy_replays_incomplete_anthropic_messages_sse() {
    let upstream_address = spawn_incomplete_anthropic_messages_sse_upstream().await;
    let shim_address = spawn_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "claude-sonnet",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("event: message_start"));
    assert!(!body.contains("event: message_stop"));
}

#[tokio::test]
async fn proxy_translates_openai_responses_stream_to_anthropic_messages_stream() {
    let upstream_address = spawn_complete_sse_upstream().await;
    let shim_address = spawn_anthropic_to_responses_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/messages"))
        .json(&json!({
            "model": "claude-sonnet",
            "max_tokens": 256,
            "messages": [{"role": "user", "content": "hello"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("\"type\":\"text_delta\""));
    assert!(body.contains("\"text\":\"hello\""));
    assert!(body.contains("event: message_stop"));
    assert!(!body.contains("event: response.output_text.delta"));
}
