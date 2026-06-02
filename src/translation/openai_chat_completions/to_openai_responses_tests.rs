use axum::body::{to_bytes, Body};
use axum::http::{header, Response};
use serde_json::{json, Value};

use super::translate_response;

#[tokio::test]
async fn translates_chat_completion_response_to_responses_shape() {
    let upstream = json!({
        "id": "chatcmpl_123",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "hello",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"id\":\"42\"}"}
                }]
            },
            "finish_reason": "tool_calls",
            "logprobs": null
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15,
            "completion_tokens_details": {"reasoning_tokens": 2}
        }
    });
    let mut response = Response::new(Body::from(serde_json::to_vec(&upstream).unwrap()));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );

    let translated = translate_response(response).await.unwrap();
    let body = to_bytes(translated.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(value["id"], "resp_chatcmpl_123");
    assert_eq!(value["object"], "response");
    assert_eq!(value["model"], "MiniMax-M3");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["output"][0]["type"], "function_call");
    assert_eq!(value["output"][0]["name"], "lookup");
    assert_eq!(value["output"][1]["type"], "message");
    assert_eq!(value["output"][1]["content"][0]["type"], "output_text");
    assert_eq!(value["output"][1]["content"][0]["text"], "hello");
    assert_eq!(value["usage"]["input_tokens"], 10);
    assert_eq!(
        value["usage"]["output_tokens_details"]["reasoning_tokens"],
        2
    );
}

#[tokio::test]
async fn translates_chat_stream_to_responses_sse() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
        "data: [DONE]\n\n"
    );
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let translated = translate_response(response).await.unwrap();
    let body = to_bytes(translated.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("event: response.created"));
    assert!(text.contains("event: response.output_item.added"));
    assert!(text.contains("event: response.output_text.delta"));
    assert!(text.contains("hel"));
    assert!(text.contains("lo"));
    assert!(text.contains("event: response.output_text.done"));
    assert!(text.contains("event: response.completed"));
    assert!(text.contains("data: [DONE]"));
}
