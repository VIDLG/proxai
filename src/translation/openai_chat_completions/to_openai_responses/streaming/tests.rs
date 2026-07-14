use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::Value;

use crate::http_support::into_byte_stream;
use crate::translation::streaming::StreamTranslationFailureSink;

use super::super::translate_streaming_response_with_failure_sink;

async fn translate_chat_stream_body(body: &'static str) -> String {
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let translated = translate_streaming_response_with_failure_sink(
        into_byte_stream(response.into_body().into_data_stream()),
        StreamTranslationFailureSink::default(),
    );
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn response_stream_payloads(body: &str) -> Vec<Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str(data).ok())
        .collect()
}

#[tokio::test]
async fn translates_chat_tool_calls_stream_to_responses_sse() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_abc\",\"type\":\"function\",\"function\":{\"name\":\"get_weather\",\"arguments\":\"\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"city\\\":\\\"SF\\\"}\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.created"));
    assert!(text.contains("\"name\":\"get_weather\""));
    assert!(text.contains("event: response.function_call_arguments.delta"));
    assert!(text.contains("event: response.function_call_arguments.done"));
    assert!(text.contains("city"));
    assert!(text.contains("SF"));
    assert!(text.contains("event: response.output_item.done"));
    assert!(text.contains("event: response.completed"));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn attaches_usage_from_trailing_usage_only_chunk() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20,\"total_tokens\":30}}\n\n",
        "data: [DONE]\n\n",
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.completed"));
    assert!(text.contains("\"input_tokens\":10"));
    assert!(text.contains("\"output_tokens\":20"));
    assert!(text.contains("\"total_tokens\":30"));
}

#[tokio::test]
async fn rejects_finish_reason_without_representable_content() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n",
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains(
        "without Responses-representable content, refusal, reasoning, or function tool calls"
    ));
}

#[tokio::test]
async fn maps_length_finish_reason_to_incomplete_response() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_length\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":\"length\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.incomplete"));
    assert!(text.contains("\"status\":\"incomplete\""));
    assert!(text.contains("\"reason\":\"max_output_tokens\""));
    assert!(!text.contains("event: response.completed"));
}

#[tokio::test]
async fn allocates_distinct_responses_indexes_for_text_and_tool_calls() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_mixed\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"checking\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_mixed\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let text = translate_chat_stream_body(body).await;

    let events = response_stream_payloads(&text);
    let added = events
        .iter()
        .filter(|event| event["type"] == "response.output_item.added")
        .collect::<Vec<_>>();
    assert_eq!(added.len(), 2);
    assert_eq!(added[0]["output_index"], 0);
    assert_eq!(added[0]["item"]["type"], "message");
    assert_eq!(added[1]["output_index"], 1);
    assert_eq!(added[1]["item"]["type"], "function_call");
    assert!(!text.contains("stream translation error"));
}

#[tokio::test]
async fn translates_chat_refusal_stream_to_responses_refusal_events() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_refusal\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"refusal\":\"cannot comply\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.refusal.delta"));
    assert!(text.contains("event: response.refusal.done"));
    let events = response_stream_payloads(&text);
    let done = events
        .iter()
        .find(|event| event["type"] == "response.output_item.done")
        .expect("refusal output item should finish");
    assert_eq!(done["item"]["type"], "message");
    assert_eq!(done["item"]["content"][0]["type"], "refusal");
    assert_eq!(done["item"]["content"][0]["refusal"], "cannot comply");
}

#[tokio::test]
async fn translates_chat_reasoning_stream_to_responses_reasoning_events() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_reasoning\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"reasoning_content\":\"think\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.reasoning_text.delta"));
    assert!(text.contains("event: response.reasoning_text.done"));
    assert!(text.contains("\"type\":\"reasoning\""));
    assert!(text.contains("\"text\":\"think\""));
}

#[tokio::test]
async fn rejects_usage_only_chunk_before_first_assistant_chunk() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
        "data: [DONE]\n\n",
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("usage-only chunk before any assistant message chunk"));
}

#[tokio::test]
async fn rejects_done_before_first_chunk() {
    let body = "data: [DONE]\n\n";
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation finish error"));
}

#[tokio::test]
async fn translates_chat_stream_to_responses_sse() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2,\"total_tokens\":3}}\n\n",
        "data: [DONE]\n\n"
    );
    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("event: response.created"));
    assert!(text.contains("event: response.output_item.added"));
    assert!(text.contains("event: response.output_text.delta"));
    assert!(text.contains("hel"));
    assert!(text.contains("lo"));
    assert!(text.contains("event: response.output_text.done"));
    assert!(text.contains("event: response.completed"));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn rejects_done_before_terminal_finish_reason() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"},\"finish_reason\":null}]}\n\n",
        "data: [DONE]\n\n"
    );

    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation finish error"));
    assert!(text.contains("emitted [DONE] before a terminal finish_reason"));
}

#[tokio::test]
async fn rejects_chat_tool_stream_without_id_on_first_chunk() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{}\"}}]},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
        "data: [DONE]\n\n"
    );

    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("started without a tool call id"));
}

#[tokio::test]
async fn rejects_stream_that_changes_id() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl_456\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n",
        "data: [DONE]\n\n"
    );

    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("Chat stream changed id from resp_chatcmpl_123 to resp_chatcmpl_456"));
}

#[tokio::test]
async fn rejects_stream_with_multiple_choices() {
    let body = concat!(
        "data: {\"id\":\"chatcmpl_123\",\"object\":\"chat.completion.chunk\",\"created\":1234,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"},\"finish_reason\":null},{\"index\":1,\"delta\":{\"content\":\"world\"},\"finish_reason\":null}]}\n\n",
        "data: [DONE]\n\n"
    );

    let text = translate_chat_stream_body(body).await;

    assert!(text.contains("stream translation error"));
    assert!(text.contains("Chat stream emitted multiple choices"));
}
