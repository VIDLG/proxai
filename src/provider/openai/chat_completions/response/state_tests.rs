use bytes::Bytes;
use serde_json::json;

use super::{ChatResponseObservation, ChatUpstreamResponseState};
use crate::http_support::ContentType;
use crate::protocol::openai::chat_completions::FinishReason;
use crate::sse::SseEventScanner;

fn observe_sse_chunk(state: &mut ChatUpstreamResponseState, chunk: &[u8]) {
    let mut scanner = SseEventScanner::default();
    let events = scanner.scan(chunk);
    state.observe_events(&events);
}

#[test]
fn state_extracts_stream_chat_completion_chunks_and_ignores_done() {
    let mut state = ChatUpstreamResponseState::default();

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

    observe_sse_chunk(&mut state, &chunk);

    let snapshot = state
        .terminal_response()
        .expect("chat completion stream snapshot");
    let ChatResponseObservation::StreamChunk(projection) = snapshot else {
        panic!("expected stream chat completion projection");
    };
    assert_eq!(projection.id, "chatcmpl_stream");
    assert_eq!(projection.object, "chat.completion.chunk");
    assert_eq!(
        projection.choices[0].finish_reason,
        Some(FinishReason::Stop)
    );
    assert_eq!(projection.usage.as_ref().expect("usage").total_tokens, 10);

    let summary = state.effective_summary();
    assert_eq!(summary.finish_reasons.get("stop"), Some(&1));
    assert_eq!(summary.tool_call_names.get("lookup"), Some(&1));
    assert!(state.stream_done);
}

#[test]
fn stream_observed_summary_deduplicates_tool_call_deltas_without_snapshot() {
    let mut state = ChatUpstreamResponseState::default();

    let chunk = Bytes::from(format!(
        "data: {}\n\ndata: {}\n\n",
        json!({
            "id": "chatcmpl_stream",
            "object": "chat.completion.chunk",
            "created": 1,
            "model": "gpt-4.1",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "o",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_456",
                        "type": "function",
                        "function": {"name": "lookup", "arguments": "{\"id"}
                    }]
                },
                "finish_reason": null
            }]
        }),
        json!({
            "id": "chatcmpl_stream",
            "object": "chat.completion.chunk",
            "created": 1,
            "model": "gpt-4.1",
            "choices": [{
                "index": 0,
                "delta": {
                    "content": "k",
                    "tool_calls": [{
                        "index": 0,
                        "function": {"arguments": "\":\"42\"}"}
                    }]
                },
                "finish_reason": null
            }]
        })
    ));

    observe_sse_chunk(&mut state, &chunk);

    assert!(state.terminal_response().is_none());
    let summary = state.effective_summary();
    assert_eq!(summary.output_items.values().sum::<u64>(), 3);
    assert_eq!(summary.tool_call_names.get("lookup"), Some(&1));
}

#[test]
fn content_type_helper_still_classifies_sse_header() {
    let value = http::HeaderValue::from_static("text/event-stream; charset=utf-8");
    let content_type = ContentType::try_from(&value).unwrap();

    assert!(content_type.is_sse());
}
