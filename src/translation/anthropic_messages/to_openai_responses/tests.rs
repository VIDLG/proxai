use serde_json::json;

use crate::http_support::into_byte_stream;
use crate::protocol::anthropic::messages::Message;
use crate::protocol::openai::responses::{Response as OpenaiResponse, ResponseStreamEvent};
use crate::sse::SseEventScanner;

use axum::body::{Body, to_bytes};
use axum::http::{Response, header};

use super::{translate_message, translate_non_streaming_payload, translate_streaming_stream};

fn assert_openai_response_stream_events_deserialize(body: &str) {
    let mut scanner = SseEventScanner::default();
    let events = scanner.scan(body.as_bytes());
    assert!(
        !events.is_empty(),
        "translated stream should contain SSE events"
    );
    for event in events {
        let payload = event
            .payload_with_type()
            .expect("translated event payload should be JSON");
        let _: ResponseStreamEvent = serde_json::from_value(payload.clone()).unwrap_or_else(|error| {
                panic!("translated event should deserialize as OpenAI Responses stream event: {error}; payload={payload}")
            });
    }
}

#[test]
fn translates_anthropic_message_to_openai_responses_shape() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_123",
        "container": null,
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "text", "text": "hello", "citations": null},
            {
                "type": "tool_use",
                "id": "toolu_1",
                "caller": {"type": "direct"},
                "name": "lookup",
                "input": {"id": "42"}
            }
        ],
        "stop_details": null,
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 6,
            "cache_creation": null,
            "cache_creation_input_tokens": null,
            "cache_read_input_tokens": 3,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": null
        }
    }))
    .unwrap();

    let translated = translate_message(&message).unwrap();
    let value = serde_json::to_value(translated).unwrap();
    let _: OpenaiResponse = serde_json::from_value(value.clone())
        .expect("translated response should deserialize as OpenAI Responses");

    assert_eq!(value["id"], "resp_msg_123");
    assert_eq!(value["object"], "response");
    assert_eq!(value["model"], "glm-5.1");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["usage"]["input_tokens"], 10);
    assert_eq!(value["usage"]["input_tokens_details"]["cached_tokens"], 3);
    assert_eq!(value["output"][0]["type"], "message");
    assert_eq!(value["output"][0]["content"][0]["type"], "output_text");
    assert_eq!(value["output"][0]["content"][0]["text"], "hello");
    assert_eq!(value["output"][1]["type"], "function_call");
    assert_eq!(value["output"][1]["call_id"], "toolu_1");
    assert_eq!(value["output"][1]["arguments"], "{\"id\":\"42\"}");
}

#[test]
fn translates_anthropic_message_payload_to_openai_responses() {
    let provider_payload = json!({
                "id": "msg_compat",
                "type": "message",
                "container": null,
                "role": "assistant",
                "model": "glm-5.1",
                "content": [
                    {
                        "type": "thinking",
                        "thinking": "plan",
                        "signature": "sig"
                    },
                    {
                        "type": "tool_use",
                        "id": "toolu_1",
                        "caller": {"type": "direct"},
                        "name": "lookup",
                        "input": {"q": "proxai"}
                    }
                ],
                "stop_details": null,
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 6,
                    "cache_creation": null,
                    "cache_creation_input_tokens": null,
                    "cache_read_input_tokens": null,
                    "inference_geo": null,
                    "server_tool_use": {
                        "web_search_requests": 1,
                        "web_fetch_requests": 0
                    },
                    "service_tier": null
                }
    });

    let value = translate_non_streaming_payload(provider_payload).unwrap();
    let _: OpenaiResponse = serde_json::from_value(value.clone())
        .expect("translated compat response should deserialize as OpenAI Responses");

    assert_eq!(value["id"], "resp_msg_compat");
    assert_eq!(value["output"][0]["type"], "reasoning");
    assert_eq!(value["output"][1]["type"], "function_call");
    assert_eq!(value["output"][1]["name"], "lookup");
}

#[tokio::test]
async fn translates_anthropic_message_stream_to_openai_responses_sse() {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(
            "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"server_tool_use\":null,\"service_tier\":null}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\",\"citations\":null}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":8,\"output_tokens\":2,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_text.delta"));
    assert!(body.contains("\"delta\":\"ok\""));
    assert!(body.contains("event: response.output_text.done"));
    assert!(body.contains("event: response.completed"));
    assert!(body.contains("\"id\":\"resp_msg_stream\""));
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn translates_thinking_stream_to_openai_responses_sse() {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(
            "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_minimax\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"MiniMax-M2.7-highspeed\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"server_tool_use\":null,\"service_tier\":null}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":8,\"output_tokens\":2,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.reasoning_summary_text.delta"));
    assert!(body.contains("\"delta\":\"plan\""));
    assert!(body.contains("event: response.reasoning_summary_text.done"));
    assert!(body.contains("event: response.completed"));
    assert!(
        !body.contains("event: error"),
        "thinking block stream must not fail translation: {body}"
    );
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn translates_provider_tool_stream_with_required_nullable_normalization() {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(
            "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_tool\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"server_tool_use\":null,\"service_tier\":null}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"lookup\",\"input\":{}}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"q\\\":\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"proxai\\\"}\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":8,\"output_tokens\":2,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":{\"web_search_requests\":1,\"web_fetch_requests\":0}}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_item.added"));
    assert!(body.contains("\"type\":\"function_call\""));
    assert!(body.contains("event: response.function_call_arguments.done"));
    assert!(body.contains("event: response.completed"));
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn translates_interrupted_thinking_then_tool_start_stream_to_parseable_events() {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(
            "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_mimo\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"mimo-v2.5-pro\",\"content\":[],\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"write_file\",\"input\":{}}}\n\n",
        ))
        .unwrap();

    let translated =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.reasoning_summary_text.delta"));
    assert!(body.contains("event: response.reasoning_summary_text.done"));
    assert!(
        body.contains("event: response.output_item.added"),
        "body={body}"
    );
    assert!(body.contains("\"type\":\"function_call\""));
    assert!(
        !body.contains("event: response.completed"),
        "interrupted upstream stream should not be translated as completed"
    );
    assert_openai_response_stream_events_deserialize(&body);
}
