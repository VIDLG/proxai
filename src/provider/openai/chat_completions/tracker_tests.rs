use axum::http::HeaderMap;
use bytes::Bytes;
use serde_json::json;

use super::{ChatUpstreamResponseTracker, ObservedChatResponse};
use crate::protocol::openai::chat_completions::{
    ChatResponseProjection, CreateChatCompletionResponse, FinishReason,
};
use crate::upstream::ContentType;

#[test]
fn tracker_extracts_non_stream_chat_completion_usage() {
    let headers = HeaderMap::new();
    let mut tracker = ChatUpstreamResponseTracker::from_headers(&headers);
    let body = json!({
        "id": "chatcmpl_123",
        "object": "chat.completion",
        "created": 1,
        "model": "gpt-4.1",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "ok",
                "tool_calls": [{
                    "type": "function",
                    "id": "call_123",
                    "function": {"name": "lookup", "arguments": "{}"}
                }]
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 3,
            "total_tokens": 13,
            "prompt_tokens_details": {"cached_tokens": 4},
            "completion_tokens_details": {"reasoning_tokens": 2}
        }
    });

    let bytes = serde_json::to_vec(&body).unwrap();
    tracker.scan_bytes(&bytes[..8]);
    tracker.scan_bytes(&bytes[8..]);
    finish_non_stream(&mut tracker);

    let ObservedChatResponse::Response(projection) = tracker
        .state
        .observed
        .latest
        .as_ref()
        .expect("chat completion projection")
    else {
        panic!("expected non-stream chat completion projection");
    };
    assert_eq!(projection.id, "chatcmpl_123");
    assert_eq!(projection.model, "gpt-4.1");
    assert_eq!(projection.choices.len(), 1);
    assert_eq!(
        projection.choices[0].finish_reason,
        Some(FinishReason::Stop)
    );
    let usage = projection.usage.as_ref().expect("usage");
    assert_eq!(usage.total_tokens, 13);
    assert_eq!(
        usage
            .prompt_tokens_details
            .and_then(|details| details.cached_tokens),
        Some(4)
    );
    assert_eq!(
        usage
            .completion_tokens_details
            .and_then(|details| details.reasoning_tokens),
        Some(2)
    );

    let summary = tracker.state.observed.effective_summary();
    assert_eq!(summary.output_items.values().sum::<u64>(), 4);
    assert_eq!(summary.finish_reasons.get("stop"), Some(&1));
    assert_eq!(summary.tool_calls.get("lookup"), Some(&1));
}

#[test]
fn tracker_extracts_stream_chat_completion_chunks_and_ignores_done() {
    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("text/event-stream"),
    );
    let mut tracker = ChatUpstreamResponseTracker::from_headers(&headers);

    let chunk = Bytes::from(format!(
        "data: {}\n\n{}\n\n",
        json!({
            "id": "chatcmpl_stream",
            "object": "chat.completion.chunk",
            "created": 1,
            "model": "gpt-4.1",
            "choices": [{
                "index": 0,
                "delta": {
                    "role": "assistant",
                    "content": "ok",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_456",
                        "type": "function",
                        "function": {"name": "lookup", "arguments": "{}"}
                    }]
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 8,
                "completion_tokens": 2,
                "total_tokens": 10
            }
        }),
        "data: [DONE]"
    ));

    tracker.scan_bytes(&chunk);

    let ObservedChatResponse::Stream(projection) = tracker
        .state
        .observed
        .latest
        .as_ref()
        .expect("chat completion chunk projection")
    else {
        panic!("expected stream chat completion projection");
    };
    assert_eq!(projection.id, "chatcmpl_stream");
    assert_eq!(projection.object, "chat.completion.chunk");
    assert_eq!(
        projection.choices[0].finish_reason,
        Some(FinishReason::Stop)
    );
    assert_eq!(projection.usage.as_ref().expect("usage").total_tokens, 10);

    let summary = tracker.state.observed.effective_summary();
    assert_eq!(summary.finish_reasons.get("stop"), Some(&1));
    assert_eq!(summary.tool_calls.get("lookup"), Some(&1));
    assert!(tracker.state.observed.done);
}

fn finish_non_stream(tracker: &mut ChatUpstreamResponseTracker) {
    if tracker.is_sse || tracker.json_body.is_empty() {
        return;
    }
    let response =
        serde_json::from_slice::<async_openai::types::chat::CreateChatCompletionResponse>(
            &tracker.json_body,
        )
        .expect("parse non-stream chat completion response");
    let projection = ChatResponseProjection::from(CreateChatCompletionResponse::from(response));
    tracker
        .state
        .observed
        .record(ObservedChatResponse::Response(projection));
    tracker.json_body.clear();
}

#[test]
fn content_type_helper_still_classifies_sse_header() {
    let value = http::HeaderValue::from_static("text/event-stream; charset=utf-8");
    let content_type = ContentType::try_from(&value).unwrap();

    assert!(content_type.is_sse());
}
