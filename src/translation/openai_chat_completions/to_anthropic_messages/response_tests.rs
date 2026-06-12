use axum::{
    body::{Body, to_bytes},
    http::Response,
};
use serde_json::{Value, json};

use crate::http_support::into_byte_stream;
use crate::protocol::anthropic::messages::{Message, MessageStreamEvent};
use crate::translation::openai_chat_completions::to_anthropic_messages::{
    translate_non_streaming_payload, translate_streaming_stream,
};

#[test]
fn translates_chat_response_to_anthropic_message() {
    let payload = json!({
        "id": "chatcmpl_123",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "hello",
                "refusal": null,
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }],
        "usage": {
            "prompt_tokens": 3,
            "completion_tokens": 2,
            "total_tokens": 5,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        },
        "service_tier": null
    });

    let translated = translate_non_streaming_payload(payload).expect("translation should succeed");
    let message: Message = serde_json::from_value(translated.clone()).unwrap_or_else(|error| {
        panic!("translated response should deserialize: {error}; payload={translated}")
    });

    assert_eq!(message.id, "msg_chatcmpl_123");
    assert_eq!(message.model, "gpt-test");
    assert_eq!(message.stop_reason.unwrap().to_string(), "end_turn");
    assert_eq!(message.usage.input_tokens, 3);
    assert_eq!(message.usage.output_tokens, 2);
    assert_eq!(message.content.len(), 1);
    assert_eq!(
        message.content[0],
        serde_json::from_value(json!({
            "type": "text",
            "text": "hello",
            "citations": null
        }))
        .unwrap()
    );
}

#[test]
fn translates_chat_function_tool_call_to_anthropic_tool_use() {
    let payload = json!({
        "id": "chatcmpl_tool",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "refusal": null,
                "tool_calls": [{
                    "type": "function",
                    "id": "call_1",
                    "function": {
                        "name": "lookup",
                        "arguments": "{\"q\":\"zed\"}"
                    }
                }],
                "annotations": null,
                "audio": null
            },
            "finish_reason": "tool_calls",
            "logprobs": null
        }],
        "usage": null,
        "service_tier": null
    });

    let translated = translate_non_streaming_payload(payload).expect("translation should succeed");
    assert_eq!(
        translated["content"][0],
        json!({
            "type": "tool_use",
            "id": "call_1",
            "caller": {"type": "direct"},
            "input": {"q": "zed"},
            "name": "lookup"
        })
    );
    assert_eq!(translated["stop_reason"], "tool_use");
}

#[test]
fn translates_chat_refusal_to_anthropic_refusal_stop_reason() {
    let payload = json!({
        "id": "chatcmpl_refusal",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "refusal": "I can't help with that.",
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }],
        "usage": null,
        "service_tier": null
    });

    let translated = translate_non_streaming_payload(payload).expect("translation should succeed");
    assert_eq!(translated["stop_reason"], "refusal");
    assert_eq!(
        translated["stop_details"],
        json!({
            "type": "refusal",
            "category": null,
            "explanation": "I can't help with that."
        })
    );
    assert_eq!(translated["stop_sequence"], Value::Null);
    assert_eq!(
        translated["content"][0],
        json!({
            "type": "text",
            "text": "I can't help with that.",
            "citations": null
        })
    );
}

#[test]
fn rejects_chat_response_with_both_content_and_refusal() {
    let payload = json!({
        "id": "chatcmpl_mixed_refusal",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "ordinary text",
                "refusal": "I can't help with that.",
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }],
        "usage": null,
        "service_tier": null
    });

    let error =
        translate_non_streaming_payload(payload).expect_err("mixed content/refusal should fail");
    assert!(
        error
            .to_string()
            .contains("contains both content and refusal")
    );
}

#[test]
fn rejects_chat_response_without_anthropic_representable_content() {
    let payload = json!({
        "id": "chatcmpl_empty",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "refusal": null,
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }],
        "usage": null,
        "service_tier": null
    });

    let error = translate_non_streaming_payload(payload).expect_err("empty response should fail");
    assert!(
        error
            .to_string()
            .contains("no Anthropic-representable content")
    );
}

#[test]
fn rejects_chat_response_with_multiple_choices() {
    let payload = json!({
        "id": "chatcmpl_multi",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "first",
                    "refusal": null,
                    "tool_calls": null,
                    "annotations": null,
                    "audio": null
                },
                "finish_reason": "stop",
                "logprobs": null
            },
            {
                "index": 1,
                "message": {
                    "role": "assistant",
                    "content": "second",
                    "refusal": null,
                    "tool_calls": null,
                    "annotations": null,
                    "audio": null
                },
                "finish_reason": "stop",
                "logprobs": null
            }
        ],
        "usage": null,
        "service_tier": null
    });

    let error =
        translate_non_streaming_payload(payload).expect_err("multi-choice response should fail");
    assert!(error.to_string().contains("has 2 choices"));
    assert!(
        error
            .to_string()
            .contains("can represent exactly one assistant message")
    );
}

#[test]
fn rejects_chat_response_non_assistant_role() {
    let payload = json!({
        "id": "chatcmpl_role",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "user",
                "content": "hello",
                "refusal": null,
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": null
        }],
        "usage": null,
        "service_tier": null
    });

    let error = translate_non_streaming_payload(payload).expect_err("role should fail");
    assert!(
        error
            .to_string()
            .contains("role user cannot be represented")
    );
}

#[test]
fn rejects_chat_response_choice_logprobs() {
    let payload = json!({
        "id": "chatcmpl_logprobs",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "hello",
                "refusal": null,
                "tool_calls": null,
                "annotations": null,
                "audio": null
            },
            "finish_reason": "stop",
            "logprobs": {"content": [], "refusal": null}
        }],
        "usage": null,
        "service_tier": null
    });

    let error = translate_non_streaming_payload(payload).expect_err("logprobs should fail");
    assert!(
        error
            .to_string()
            .contains("choice logprobs cannot be represented")
    );
}

#[tokio::test]
async fn translates_chat_stream_to_anthropic_messages_sse() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":null,\"tool_calls\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("stream should translate");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("event: message_start"));
    let payloads = parse_sse_payloads(&text);
    let start = payloads
        .iter()
        .find(|payload| payload["type"] == "content_block_start")
        .expect("stream should start a text content block");
    assert_eq!(start["content_block"]["type"], "text");
    assert_eq!(start["content_block"]["text"], "ok");
    assert!(text.contains("event: content_block_stop"));
    assert!(text.contains("\"stop_reason\":\"end_turn\""));
    assert!(text.contains("event: message_stop"));

    for event in text.split("\n\n").filter(|event| event.contains("data: ")) {
        let data = event
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("event should have data");
        let payload: Value = serde_json::from_str(data).expect("event data should be JSON");
        let _: MessageStreamEvent = serde_json::from_value(payload.clone()).unwrap_or_else(|error| {
            panic!("translated event should deserialize as Anthropic stream event: {error}; payload={payload}")
        });
    }
}

#[tokio::test]
async fn translates_chat_refusal_stream_to_anthropic_refusal_stop_reason() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_refusal_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":null,\"tool_calls\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_refusal_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":\"I can't help with that.\"},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_refusal_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("stream should translate");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    let payloads = parse_sse_payloads(&text);
    let start = payloads
        .iter()
        .find(|payload| payload["type"] == "content_block_start")
        .expect("stream should start a refusal text content block");
    assert_eq!(start["content_block"]["type"], "text");
    assert_eq!(start["content_block"]["text"], "I can't help with that.");
    assert!(text.contains("\"stop_reason\":\"refusal\""));
    assert!(text.contains("\"stop_details\":{"));
    assert!(text.contains("\"type\":\"refusal\""));
    assert!(text.contains("\"category\":null"));
    assert!(text.contains("\"explanation\":\"I can't help with that.\""));
}

#[tokio::test]
async fn delays_chat_stream_stop_until_usage_only_chunk() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_usage_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_usage_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":{\"prompt_tokens\":99,\"completion_tokens\":88,\"total_tokens\":187,\"prompt_tokens_details\":null,\"completion_tokens_details\":null},\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_usage_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":1,\"total_tokens\":5,\"prompt_tokens_details\":null,\"completion_tokens_details\":null},\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("stream should translate");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("\"input_tokens\":4"));
    assert!(text.contains("\"output_tokens\":1"));
    assert!(!text.contains("\"input_tokens\":99"));
    assert!(!text.contains("\"output_tokens\":88"));
    assert_eq!(text.matches("event: message_stop").count(), 1);
}

#[tokio::test]
async fn rejects_chat_stream_done_before_finish_reason() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_done_without_finish\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation finish error"));
    assert!(text.contains("emitted [DONE] before a terminal finish_reason"));
}

#[tokio::test]
async fn rejects_chat_stream_eof_before_finish_reason() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_eof_without_finish\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation finish error"));
    assert!(text.contains("reached EOF before a terminal finish_reason"));
}

#[tokio::test]
async fn rejects_chat_stream_chunk_with_multiple_choices() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_multi_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null},{\"index\":1,\"delta\":{\"content\":\"second\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("has 2 choices"));
    assert!(text.contains("can represent exactly one assistant message"));
}

#[tokio::test]
async fn rejects_chat_stream_that_changes_id() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_a\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_b\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"second\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("changed id from msg_chatcmpl_a to msg_chatcmpl_b"));
}

#[tokio::test]
async fn rejects_chat_stream_that_changes_model() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_model\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-a\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_model\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-b\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"second\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("changed model from gpt-a to gpt-b"));
}

#[tokio::test]
async fn rejects_chat_stream_that_switches_choice_index() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_switch_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_switch_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":1,\"delta\":{\"content\":\"second\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("switched from choice index 0 to 1"));
}

#[tokio::test]
async fn rejects_chat_stream_choice_after_message_stop() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":1,\"total_tokens\":5,\"prompt_tokens_details\":null,\"completion_tokens_details\":null},\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_after_stop\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"late\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("after the Anthropic message was stopped"));
}

#[tokio::test]
async fn rejects_chat_stream_with_both_content_and_refusal() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_mixed_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ordinary\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_mixed_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":\"I can't help with that.\"},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("contains both content and refusal"));
}

#[tokio::test]
async fn rejects_chat_stream_without_anthropic_representable_content() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_empty\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("without Anthropic-representable content"));
}

#[tokio::test]
async fn puts_first_chat_text_delta_in_content_block_start() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_first_text\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_first_text\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_first_text\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"stop\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("stream should translate");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");
    let payloads = parse_sse_payloads(&text);

    let start = payloads
        .iter()
        .find(|payload| payload["type"] == "content_block_start")
        .expect("stream should start a content block");
    assert_eq!(start["content_block"]["type"], "text");
    assert_eq!(start["content_block"]["text"], "hel");

    let delta = payloads
        .iter()
        .find(|payload| payload["type"] == "content_block_delta")
        .expect("stream should emit a later text delta");
    assert_eq!(delta["delta"]["type"], "text_delta");
    assert_eq!(delta["delta"]["text"], "lo");
}

#[tokio::test]
async fn rejects_chat_stream_usage_only_chunk_before_finish_reason() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_usage_too_early\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_usage_too_early\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":1,\"total_tokens\":5,\"prompt_tokens_details\":null,\"completion_tokens_details\":null},\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("usage-only chunk before a terminal finish_reason"));
}

#[tokio::test]
async fn rejects_chat_stream_choice_logprobs() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_logprobs\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":{\"content\":[],\"refusal\":null}}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("choice logprobs cannot be represented"));
}

#[tokio::test]
async fn rejects_chat_stream_non_assistant_delta_role() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_role\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\",\"tool_calls\":null,\"role\":\"user\",\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("translation errors are encoded as SSE error events");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");

    assert!(text.contains("stream translation error"));
    assert!(text.contains("delta role user cannot be represented"));
}

#[tokio::test]
async fn translates_chat_stream_tool_arguments_as_input_json_delta() {
    let response = Response::builder()
        .header("content-type", "text/event-stream")
        .body(Body::from(
            "data: {\"id\":\"chatcmpl_tool_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{\\\"q\\\"\"}}],\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_tool_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":[{\"index\":0,\"id\":null,\"type\":null,\"function\":{\"name\":null,\"arguments\":\":\\\"zed\\\"}\"}}],\"role\":null,\"refusal\":null},\"finish_reason\":null,\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: {\"id\":\"chatcmpl_tool_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":null,\"tool_calls\":null,\"role\":null,\"refusal\":null},\"finish_reason\":\"tool_calls\",\"logprobs\":null}],\"usage\":null,\"service_tier\":null}\n\n\
data: [DONE]\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .expect("stream should translate");
    let text = String::from_utf8(body.to_vec()).expect("translated SSE should be UTF-8");
    let payloads = parse_sse_payloads(&text);

    let start = payloads
        .iter()
        .find(|payload| payload["type"] == "content_block_start")
        .expect("stream should start a tool block");
    assert_eq!(start["content_block"]["type"], "tool_use");
    assert_eq!(start["content_block"]["id"], "call_1");
    assert_eq!(start["content_block"]["name"], "lookup");
    assert_eq!(start["content_block"]["input"], json!({}));

    let partials: Vec<_> = payloads
        .iter()
        .filter(|payload| payload["type"] == "content_block_delta")
        .map(|payload| payload["delta"].clone())
        .collect();
    assert_eq!(partials.len(), 2);
    assert!(
        partials
            .iter()
            .all(|delta| delta["type"] == "input_json_delta")
    );
    assert_eq!(partials[0]["partial_json"], "{\"q\"");
    assert_eq!(partials[1]["partial_json"], ":\"zed\"}");
    assert!(text.contains("\"stop_reason\":\"tool_use\""));
}

fn parse_sse_payloads(text: &str) -> Vec<Value> {
    text.split("\n\n")
        .filter_map(|event| {
            event
                .lines()
                .find_map(|line| line.strip_prefix("data: "))
                .map(|data| serde_json::from_str(data).expect("event data should be JSON"))
        })
        .collect()
}
