use axum::body::{Body, to_bytes};
use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;

use super::{normalize_sse_stream, normalize_stream_event_payload};

#[test]
fn fills_missing_finish_reason_with_null() {
    let normalized = normalize_stream_event_payload(json!({
        "id": "chatcmpl_minimax",
        "object": "chat.completion.chunk",
        "created": 1,
        "model": "MiniMax-M3",
        "choices": [{
            "index": 0,
            "delta": {"content": "hello"}
        }]
    }));

    assert_eq!(normalized["choices"][0]["finish_reason"], json!(null));
    serde_json::from_value::<CreateChatCompletionStreamResponse>(normalized)
        .expect("normalized compatible Chat chunk should match the official wire model");
}

#[test]
fn preserves_explicit_finish_reason_values() {
    for finish_reason in [json!(null), json!("stop")] {
        let normalized = normalize_stream_event_payload(json!({
            "choices": [{
                "index": 0,
                "delta": {},
                "finish_reason": finish_reason
            }]
        }));

        assert_eq!(normalized["choices"][0]["finish_reason"], finish_reason);
    }
}

#[tokio::test]
async fn normalizes_chat_sse_without_changing_done_sentinel() {
    let input = futures_util::stream::iter([Ok::<_, std::io::Error>(
        axum::body::Bytes::from_static(
            b"data: {\"id\":\"chatcmpl_minimax\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n\n",
        ),
    )]);

    let body = to_bytes(Body::from_stream(normalize_sse_stream(input)), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("\"finish_reason\":null"));
    assert!(text.ends_with("data: [DONE]\n\n"));
    assert!(!text.contains("event: message"));
}
