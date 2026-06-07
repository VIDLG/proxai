use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::json;

use crate::http_support::into_byte_stream;

use super::translate_streaming_stream;

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

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();

    let chunks = chat_stream_payloads(body);

    assert!(body.contains("event: message"));
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");
    assert_eq!(chunks[0]["choices"][0]["delta"]["role"], "assistant");
    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "hello");
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 1,
            "total_tokens": 3,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        })
    );
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_anthropic_stream_refusal_details_to_chat_refusal_delta() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_refusal_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"refusal\",\"stop_sequence\":null,\"stop_details\":{\"type\":\"refusal\",\"category\":\"cyber\",\"explanation\":\"I can't help with cyber abuse.\"},\"container\":null},\"usage\":{\"input_tokens\":2,\"output_tokens\":0,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    let chunks = chat_stream_payloads(body);

    assert_eq!(
        chunks[1]["choices"][0]["finish_reason"],
        serde_json::Value::Null
    );
    assert_eq!(
        chunks[1]["choices"][0]["delta"]["refusal"],
        "I can't help with cyber abuse."
    );
    assert!(chunks[2]["choices"][0]["delta"]["content"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["refusal"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["role"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["tool_calls"].is_null());
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 0,
            "total_tokens": 2,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        })
    );
}

#[tokio::test]
async fn maps_anthropic_tool_block_indexes_to_chat_tool_call_indexes() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_tool_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"intro\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"lookup\",\"input\":{}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\":\\\"proxai\\\"}\"}}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    let chunks = chat_stream_payloads(body);

    assert_eq!(
        chunks[2]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
    assert_eq!(
        chunks[3]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
}

fn chat_stream_payloads(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}
