use super::common::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::Arc;

#[tokio::test]
async fn proxy_forwards_openai_chat_completions_without_conversion() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_chat_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/chat/completions"))
        .header("authorization", "Bearer dummy")
        .header("x-api-key", "client-anthropic-key")
        .json(&json!({
            "model": "gpt-4.1",
            "messages": [
                {"role": "system", "content": "You are terse."},
                {"role": "user", "content": "Reply ok."}
            ],
            "stream": false,
            "tools": [{
                "type": "function",
                "function": {
                    "name": "lookup",
                    "parameters": {"type": "object"}
                }
            }]
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
    assert_eq!(payloads[0]["model"], "gpt-4.1");
    assert_eq!(payloads[0]["messages"].as_array().unwrap().len(), 2);
    assert_eq!(payloads[0]["tools"][0]["function"]["name"], "lookup");

    let authorizations = capture.authorizations.lock().await;
    assert_eq!(
        authorizations.as_slice(),
        &[Some("Bearer test-upstream-key".to_string())]
    );
    let api_keys = capture.api_keys.lock().await;
    assert_eq!(api_keys.as_slice(), &[None]);

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/chat/completions".to_string()]);
}

#[tokio::test]
async fn proxy_preserves_useful_upstream_error_headers_for_openai_chat_completions() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_error_upstream(capture.clone()).await;
    let shim_address = spawn_chat_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/chat/completions"))
        .json(&json!({
            "model": "gpt-4.1",
            "messages": [{"role": "user", "content": "Hello"}],
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
            .get("x-request-id")
            .and_then(|value| value.to_str().ok()),
        Some("req_test_123")
    );
    assert_eq!(
        response
            .headers()
            .get("x-ratelimit-remaining-requests")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("upstream 429: quota exhausted"));

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/chat/completions".to_string()]);
}

#[tokio::test]
async fn proxy_forwards_openai_chat_completion_sse_without_changing_body() {
    let upstream_address = spawn_chat_completion_sse_upstream().await;
    let shim_address = spawn_chat_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/chat/completions"))
        .json(&json!({
            "model": "gpt-4.1",
            "messages": [{"role": "user", "content": "Hello"}],
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
    assert!(body.contains("chat.completion.chunk"));
    assert!(body.contains("data: [DONE]"));
    assert!(body.contains("\"finish_reason\":"));
}

#[tokio::test]
async fn proxy_replays_incomplete_openai_chat_completion_sse() {
    let upstream_address = spawn_incomplete_chat_completion_sse_upstream().await;
    let shim_address = spawn_chat_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/chat/completions"))
        .json(&json!({
            "model": "gpt-4.1",
            "messages": [{"role": "user", "content": "Hello"}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("chat.completion.chunk"));
    assert!(body.contains("partial"));
    assert!(!body.contains("data: [DONE]"));
}
