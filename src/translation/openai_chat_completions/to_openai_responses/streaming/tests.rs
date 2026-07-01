use axum::body::{Body, to_bytes};
use axum::http::{Response, header};

use crate::http_support::into_byte_stream;
use crate::translation::streaming::translate_sse_stream;

use super::ResponsesStreamTranslator;

async fn translate_chat_stream_body(body: &'static str) -> String {
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let translated = translate_sse_stream(
        into_byte_stream(response.into_body().into_data_stream()),
        ResponsesStreamTranslator::default(),
    );
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
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
    assert!(text.contains("without Responses-representable content or function tool calls"));
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
