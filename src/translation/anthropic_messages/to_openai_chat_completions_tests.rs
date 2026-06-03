use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::json;

use super::{translate_non_streaming_response, translate_streaming_response};

#[tokio::test]
async fn translates_anthropic_message_to_chat_completion_shape() {
    let upstream = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "text", "text": "hello"},
            {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {"query": "proxai"}}
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 5}
    });
    let mut response = Response::new(Body::from(serde_json::to_vec(&upstream).unwrap()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );

    let response = translate_non_streaming_response(response).await.unwrap();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let translated: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(translated["object"], "chat.completion");
    assert_eq!(translated["id"], "chatcmpl_msg_123");
    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["choices"][0]["message"]["role"], "assistant");
    assert_eq!(translated["choices"][0]["message"]["content"], "hello");
    assert_eq!(translated["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        translated["choices"][0]["message"]["tool_calls"][0],
        json!({
            "id": "toolu_1",
            "type": "function",
            "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
        })
    );
    assert_eq!(
        translated["usage"],
        json!({
            "prompt_tokens": 3,
            "completion_tokens": 5,
            "total_tokens": 8
        })
    );
}

#[tokio::test]
async fn translates_anthropic_stream_to_chat_completion_sse() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":2,\"output_tokens\":1,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response = translate_streaming_response(response).await.unwrap();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = std::str::from_utf8(&body).unwrap();

    assert!(body.contains("event: message"));
    assert!(body.contains("\"object\":\"chat.completion.chunk\""));
    assert!(body.contains("\"delta\":{\"role\":\"assistant\"}"));
    assert!(body.contains("\"delta\":{\"content\":\"hello\"}"));
    assert!(body.contains("\"finish_reason\":\"stop\""));
    assert!(body.contains("data: [DONE]"));
}
