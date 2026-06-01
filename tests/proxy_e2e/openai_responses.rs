use super::common::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;

#[tokio::test]
async fn proxy_moves_system_to_instructions_and_overrides_authorization() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_shim(upstream_address).await;

    let client = local_client();
    let response = client
        .post(format!("http://{shim_address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer dummy")
        .header("x-api-key", "client-anthropic-key")
        .json(&json!({
            "model": "gpt-5.5",
            "input": [
                {
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": "You are a terse test assistant."}]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Reply with ok."}]
                }
            ],
            "stream": true
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
    assert_eq!(
        payloads[0]["instructions"],
        "You are a terse test assistant."
    );
    assert_eq!(payloads[0]["input"].as_array().unwrap().len(), 1);
    assert_eq!(payloads[0]["input"][0]["role"], "user");

    let authorizations = capture.authorizations.lock().await;
    assert_eq!(
        authorizations.as_slice(),
        &[Some("Bearer test-upstream-key".to_string())]
    );
    let api_keys = capture.api_keys.lock().await;
    assert_eq!(api_keys.as_slice(), &[None]);

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/responses".to_string()]);
}

#[tokio::test]
async fn proxy_does_not_duplicate_provider_api_root_path() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_shim_with_base_path(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        "/v1",
    )
    .await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer dummy")
        .json(&responses_request(false))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/responses".to_string()]);
}

#[tokio::test]
async fn proxy_preserves_useful_upstream_error_headers_for_openai_responses() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_error_upstream(capture.clone()).await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(false))
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
    assert_eq!(paths.as_slice(), &["/v1/responses".to_string()]);
}

#[tokio::test]
async fn proxy_can_capture_zed_request_case_without_header_secrets() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture).await;
    let capture_dir = unique_capture_dir();
    let shim_address = spawn_shim_with_capture(upstream_address, Some(capture_dir.clone())).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .header("authorization", "Bearer should-not-be-written")
        .json(&json!({
            "model": "gpt-5.5",
            "input": [
                {
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": "Capture this instruction."}]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Hello"}]
                }
            ],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let files = capture_files(&capture_dir).await;

    let metadata = read_json_file(&files, "inbound-request.metadata").await;
    assert_eq!(metadata["headers"]["authorization"], "<redacted>");

    let normalized = read_json_file(&files, "forwarded-request.body").await;
    assert_eq!(normalized["instructions"], "Capture this instruction.");
    assert_eq!(normalized["input"].as_array().unwrap().len(), 1);

    fs::remove_dir_all(capture_dir).await.unwrap();
}

#[tokio::test]
async fn proxy_can_capture_upstream_json_response_body() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture).await;
    let capture_dir = unique_capture_dir();
    let shim_address = spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        Some(capture_dir.clone()),
        false,
        true,
        None,
    )
    .await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(false))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.bytes().await.unwrap();
    let files = capture_files(&capture_dir).await;

    let upstream_headers = read_json_file(&files, "upstream-response.headers").await;
    assert_eq!(upstream_headers["status"], 200);
    assert_eq!(upstream_headers["content_type"], "application/json");

    let upstream_raw = read_text_file(&files, "upstream-response.body.bin").await;
    assert_eq!(upstream_raw.as_bytes(), body.as_ref());

    fs::remove_dir_all(capture_dir).await.unwrap();
}

#[tokio::test]
async fn proxy_can_capture_upstream_sse_response_body() {
    let upstream_address = spawn_response_metadata_sse_upstream().await;
    let capture_dir = unique_capture_dir();
    let shim_address = spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        Some(capture_dir.clone()),
        false,
        true,
        None,
    )
    .await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    let files = capture_files(&capture_dir).await;

    let upstream_headers = read_json_file(&files, "upstream-response.headers").await;
    assert_eq!(upstream_headers["status"], 200);
    assert_eq!(upstream_headers["content_type"], "text/event-stream");

    let upstream_raw = read_text_file(&files, "upstream-response.body.sse").await;
    assert_eq!(upstream_raw, body);

    fs::remove_dir_all(capture_dir).await.unwrap();
}

#[tokio::test]
async fn proxy_rejects_openai_responses_requests_without_model() {
    let upstream_address = spawn_upstream(Arc::new(Capture::default())).await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&json!({"input": [], "stream": true}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = response.json::<Value>().await.unwrap();
    assert_eq!(
        body["error"]["message"],
        "OpenAI Responses requests must include a non-empty `model`."
    );
    assert_eq!(body["error"]["type"], "invalid_request_error");
}

#[tokio::test]
async fn proxy_preserves_null_session_id_in_tool_argument_streams() {
    let upstream_address = spawn_null_session_id_tool_argument_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("\\\"session_id\\\":null"));
    assert!(!body.contains("\\\"session_id\\\":\\\"\\\""));
    assert_eq!(
        body,
        "event: response.function_call_arguments.delta\ndata: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"session_id\\\":null,\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":1}\n\n\
         event: response.function_call_arguments.delta\ndata: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"\\\"message\\\":\\\"hi\\\"}\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":2}\n\n\
         event: response.function_call_arguments.done\ndata: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":3}\n\n\
         event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{}}\n\n"
    );
}

#[tokio::test]
async fn proxy_preserves_empty_session_id_in_tool_argument_streams() {
    let upstream_address = spawn_empty_session_id_tool_argument_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("\\\"session_id\\\":\\\"\\\""));
    assert!(!body.contains("\\\"session_id\\\":null"));
    assert_eq!(
        body,
        "event: response.function_call_arguments.delta\ndata: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"session_id\\\":\\\"\\\",\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":1}\n\n\
         event: response.function_call_arguments.delta\ndata: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"\\\"message\\\":\\\"hi\\\"}\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":2}\n\n\
         event: response.function_call_arguments.done\ndata: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":3}\n\n\
         event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{}}\n\n"
    );
}

#[tokio::test]
async fn proxy_preserves_sse_stream_body_and_content_type() {
    let upstream_address = spawn_complete_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
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
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let body = response.text().await.unwrap();
    assert_eq!(
        body,
        "event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n\
event: response.completed\ndata: {}\n\n"
    );
}

#[tokio::test]
async fn proxy_replays_incomplete_openai_responses_sse() {
    let upstream_address = spawn_incomplete_response_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert_eq!(
        body,
        "event: response.output_text.delta\ndata: {\"delta\":\"partial\"}\n\n"
    );
    assert!(!body.contains("event: response.completed"));
}

#[tokio::test]
async fn proxy_replays_incomplete_tool_argument_stream() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_incomplete_tool_argument_sse_upstream(capture.clone()).await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&json!({
            "model": "gpt-5.4",
            "input": [
                {
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": "Use tools when edits are required."}]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Edit the file."}]
                }
            ],
            "tools": [{"type": "function", "name": "edit_file", "parameters": {"type": "object"}}],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("event: response.function_call_arguments.delta"));
    assert!(!body.contains("event: response.function_call_arguments.done"));
    assert!(!body.contains("event: response.completed"));

    let payloads = capture.payloads.lock().await;
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads[0]["instructions"],
        "Use tools when edits are required."
    );
    assert_eq!(payloads[0]["input"].as_array().unwrap().len(), 1);
    assert_eq!(payloads[0]["input"][0]["role"], "user");
    assert_eq!(payloads[0]["tools"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn proxy_injects_error_when_sse_tool_arguments_stall() {
    let upstream_address = spawn_stalled_tool_argument_sse_upstream().await;
    let shim_address = spawn_shim_with_capture_and_timeout(
        upstream_address,
        None,
        Some(Duration::from_millis(50)),
    )
    .await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("event: response.function_call_arguments.delta"));
    assert!(body.contains("event: error"));
    assert!(body.contains("upstream SSE stalled while streaming tool arguments"));
    assert!(!body.contains("event: response.function_call_arguments.done"));
    assert!(!body.contains("event: response.completed"));
}

#[tokio::test]
async fn proxy_scans_unicode_sse_without_panicking() {
    let upstream_address = spawn_unicode_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("response.output_text.delta"));
    assert!(body.contains("response.completed"));
}

#[tokio::test]
async fn proxy_forwards_response_metadata_sse_without_changing_body() {
    let upstream_address = spawn_response_metadata_sse_upstream().await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-ratelimit-remaining-requests")
            .and_then(|value| value.to_str().ok()),
        Some("9")
    );
    assert_eq!(
        response
            .headers()
            .get("x-codex-primary-used-percent")
            .and_then(|value| value.to_str().ok()),
        Some("42.5")
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("response.output_item.done"));
    assert!(body.contains("reasoning_tokens"));
    assert!(body.contains("response.completed"));
}

#[tokio::test]
async fn proxy_translates_anthropic_messages_stream_to_openai_responses_stream() {
    let upstream_address = spawn_anthropic_messages_sse_upstream().await;
    let shim_address = spawn_responses_to_anthropic_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&responses_request(true))
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
    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_text.delta"));
    assert!(body.contains("\"delta\":\"ok\""));
    assert!(body.contains("event: response.completed"));
    assert!(!body.contains("event: message_start"));
}
