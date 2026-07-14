use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::json;

use super::common::*;

// Trigger: MiniMax-M3 emitted OpenAI-compatible Chat stream chunks that omitted the
// required-nullable `choices[].finish_reason` field before the terminal chunk.
// Symptom: Responses -> Chat streaming returned `stream JSON conversion failed:
// missing field finish_reason` after the request had already logged `fwd`.
// Provenance: local Zed/proxai request `kjpuu8`, 2026-07-14; prompt and tool names sanitized.
#[tokio::test]
async fn regression_minimax_chat_stream_missing_finish_reason() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_minimax_chat_completion_sse_upstream(capture.clone()).await;
    let shim_address = spawn_responses_to_chat_shim(upstream_address).await;

    let response = local_client()
        .post(format!("http://{shim_address}/v1/responses"))
        .json(&json!({
            "model": "MiniMax-M3/high",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            }],
            "stream": true,
            "tools": [{
                "type": "function",
                "name": "lookup",
                "description": "Look up a sanitized value",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }]
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.text().await.unwrap();
    assert!(body.contains("event: response.created"), "SSE body: {body}");
    assert!(
        body.contains("event: response.output_text.delta"),
        "SSE body: {body}"
    );
    assert!(body.contains("\"delta\":\"hello\""), "SSE body: {body}");
    assert!(
        body.contains("event: response.completed"),
        "SSE body: {body}"
    );
    assert!(
        !body.contains("stream translation error"),
        "SSE body: {body}"
    );

    let payloads = capture.payloads.lock().await;
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["model"], "MiniMax-M3");
    assert_eq!(payloads[0]["stream"], true);
    assert_eq!(payloads[0]["tools"][0]["function"]["name"], "lookup");
    assert_eq!(payloads[0]["tools"][0]["function"]["strict"], json!(null));

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/chat/completions".to_string()]);
}
