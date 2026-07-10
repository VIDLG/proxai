use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::Value;

use crate::http_support::into_byte_stream;
use crate::translation::streaming::translate_sse_stream;

use super::ChatCompletionStreamTranslator;

async fn translate_responses_stream_body(body: &'static str) -> String {
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let translated = translate_sse_stream(
        into_byte_stream(response.into_body().into_data_stream()),
        ChatCompletionStreamTranslator::default(),
    );
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

fn chat_stream_payloads(body: &str) -> Vec<Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .filter_map(|data| serde_json::from_str(data).ok())
        .collect()
}

#[tokio::test]
async fn translates_responses_text_stream_to_chat_completions_sse() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":0,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":8}}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":2,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"hel\"}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"lo\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":2,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":10}}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"role\":\"assistant\""));
    assert!(text.contains("\"content\":\"hel\""));
    assert!(text.contains("\"content\":\"lo\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_responses_tool_calls_stream_to_chat_completions_sse() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_456\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_abc\",\"name\":\"get_weather\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":3,\"item_id\":\"call_abc\",\"output_index\":0,\"delta\":\"{\\\"city\\\":\\\"SF\\\"}\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_456\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"name\":\"get_weather\""));
    assert!(text.contains("\"id\":\"call_abc\""));
    assert!(text.contains("city"));
    assert!(text.contains("SF"));
    assert!(text.contains("\"finish_reason\":\"tool_calls\""));
}

#[tokio::test]
async fn translates_responses_incomplete_to_length_finish_reason() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_789\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":2,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"partial\"}\n\n",
        "event: response.incomplete\n",
        "data: {\"type\":\"response.incomplete\",\"sequence_number\":3,\"response\":{\"id\":\"resp_789\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"incomplete\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"finish_reason\":\"length\""));
}

#[tokio::test]
async fn translates_responses_refusal_to_chat_refusal_delta() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.refusal.delta\n",
        "data: {\"type\":\"response.refusal.delta\",\"sequence_number\":2,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"cannot comply\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":3,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"refusal\":\"cannot comply\""));
    assert!(text.contains("\"finish_reason\":\"stop\""));
    assert!(text.contains("data: [DONE]"));
}

#[tokio::test]
async fn compacts_responses_output_index_to_chat_tool_call_index() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_tool_index\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n",
        "event: response.output_item.added\n",
        "data: {\"type\":\"response.output_item.added\",\"sequence_number\":3,\"output_index\":1,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"lookup\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n",
        "event: response.function_call_arguments.delta\n",
        "data: {\"type\":\"response.function_call_arguments.delta\",\"sequence_number\":4,\"item_id\":\"fc_1\",\"output_index\":1,\"delta\":\"{}\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_tool_index\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    let chunks = chat_stream_payloads(&text);
    let tool_indexes = chunks
        .iter()
        .filter_map(|chunk| chunk["choices"][0]["delta"]["tool_calls"][0]["index"].as_u64())
        .collect::<Vec<_>>();
    assert!(!tool_indexes.is_empty());
    assert!(tool_indexes.iter().all(|index| *index == 0));
}

#[tokio::test]
async fn skips_unknown_responses_events_without_breaking_translation() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_unknown\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.future_progress\n",
        "data: {\"type\":\"response.future_progress\",\"sequence_number\":2,\"detail\":\"new upstream telemetry\"}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"ok\"}\n\n",
        "event: response.completed\n",
        "data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_unknown\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"content\":\"ok\""));
    assert!(text.contains("data: [DONE]"));
    assert!(!text.contains("stream translation error"));
}

#[tokio::test]
async fn reports_unexpected_eof_before_responses_terminal_event() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_eof\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: response.output_text.delta\n",
        "data: {\"type\":\"response.output_text.delta\",\"sequence_number\":2,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"partial\"}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("\"content\":\"partial\""));
    assert!(text.contains("stream translation finish error"));
    assert!(text.contains("Responses stream reached EOF before a terminal response event"));
    assert!(!text.contains("data: [DONE]"));
}

#[tokio::test]
async fn propagates_response_error_as_stream_error() {
    let body = concat!(
        "event: response.created\n",
        "data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_error\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
        "event: error\n",
        "data: {\"type\":\"error\",\"sequence_number\":2,\"message\":\"upstream failed\"}\n\n"
    );
    let text = translate_responses_stream_body(body).await;

    assert!(text.contains("event: error"));
    assert!(text.contains("Responses stream error"));
    assert!(text.contains("upstream failed"));
    assert!(!text.contains("\"finish_reason\":\"stop\""));
    assert!(!text.contains("data: [DONE]"));
}
