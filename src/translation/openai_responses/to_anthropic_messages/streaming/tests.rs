use axum::body::{Body, to_bytes};
use axum::http::{Response, header};

use crate::http_support::into_byte_stream;

use super::translate_streaming_response;

async fn translate_body(body: &'static str) -> String {
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from(body))
        .unwrap();
    let translated =
        translate_streaming_response(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(translated), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn translates_openai_responses_stream_to_anthropic_messages_sse() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":0,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":8}}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":3,\"output_index\":0,\"content_index\":0,\"item_id\":\"msg_1\",\"delta\":\"ok\"}\n\n\
event: response.output_text.done\n\
data: {\"type\":\"response.output_text.done\",\"sequence_number\":4,\"output_index\":0,\"content_index\":0,\"item_id\":\"msg_1\",\"text\":\"ok\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":2,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":10}}}\n\n",
    )
    .await;

    assert!(body.contains("event: message_start"));
    assert!(body.contains("\"id\":\"resp_123\""));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("\"type\":\"text_delta\""));
    assert!(body.contains("\"text\":\"ok\""));
    assert!(body.contains("event: message_delta"));
    assert!(body.contains("\"stop_reason\":\"end_turn\""));
    assert!(body.contains("event: message_stop"));
}

#[tokio::test]
async fn translates_reasoning_summary_stream_to_thinking_block() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_reasoning\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"reasoning\",\"id\":\"rs_1\",\"summary\":[],\"status\":\"in_progress\"}}\n\n\
event: response.reasoning_summary_part.added\n\
data: {\"type\":\"response.reasoning_summary_part.added\",\"sequence_number\":3,\"item_id\":\"rs_1\",\"output_index\":0,\"summary_index\":0,\"part\":{\"type\":\"summary_text\",\"text\":\"\"}}\n\n\
event: response.reasoning_summary_text.delta\n\
data: {\"type\":\"response.reasoning_summary_text.delta\",\"sequence_number\":4,\"item_id\":\"rs_1\",\"output_index\":0,\"summary_index\":0,\"delta\":\"thought\"}\n\n\
event: response.reasoning_summary_text.done\n\
data: {\"type\":\"response.reasoning_summary_text.done\",\"sequence_number\":5,\"item_id\":\"rs_1\",\"output_index\":0,\"summary_index\":0,\"text\":\"thought\"}\n\n\
event: response.reasoning_summary_part.done\n\
data: {\"type\":\"response.reasoning_summary_part.done\",\"sequence_number\":6,\"item_id\":\"rs_1\",\"output_index\":0,\"summary_index\":0,\"part\":{\"type\":\"summary_text\",\"text\":\"thought\"}}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":7,\"response\":{\"id\":\"resp_reasoning\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n",
    )
    .await;

    assert!(body.contains("event: content_block_start"));
    assert!(body.contains("\"type\":\"thinking\""));
    assert!(body.contains("event: content_block_delta"));
    assert!(body.contains("\"type\":\"thinking_delta\""));
    assert!(body.contains("\"thinking\":\"thought\""));
    assert!(body.contains("event: content_block_stop"));
}

#[tokio::test]
async fn rejects_output_text_delta_before_output_item_added() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_123\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\",\"usage\":{\"input_tokens\":8,\"input_tokens_details\":{\"cached_tokens\":0},\"output_tokens\":0,\"output_tokens_details\":{\"reasoning_tokens\":0},\"total_tokens\":8}}}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":2,\"output_index\":0,\"content_index\":0,\"item_id\":\"msg_1\",\"delta\":\"ok\"}\n\n",
    )
    .await;

    assert!(body.contains("stream translation error"));
    assert!(body.contains(
        "Responses stream emitted response.output_text.delta for output_index 0 before response.output_item.added"
    ));
}

#[tokio::test]
async fn reports_unexpected_eof_before_responses_terminal_event() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_eof\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n",
    )
    .await;

    assert!(body.contains("event: message_start"));
    assert!(body.contains("stream translation finish error"));
    assert!(body.contains("Responses stream reached EOF before a terminal response event"));
    assert!(!body.contains("event: message_stop"));
}

#[tokio::test]
async fn propagates_response_error_as_stream_error() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_error\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: error\n\
data: {\"type\":\"error\",\"sequence_number\":2,\"message\":\"upstream failed\"}\n\n",
    )
    .await;

    assert!(body.contains("event: message_start"));
    assert!(body.contains("event: error"));
    assert!(body.contains("Responses stream error"));
    assert!(body.contains("upstream failed"));
    assert!(!body.contains("event: message_delta"));
    assert!(!body.contains("event: message_stop"));
}

#[tokio::test]
async fn translates_multiple_message_content_parts_to_distinct_anthropic_blocks() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_multi\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n\
event: response.content_part.added\n\
data: {\"type\":\"response.content_part.added\",\"sequence_number\":3,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"\",\"annotations\":[]}}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":4,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"delta\":\"first\"}\n\n\
event: response.output_text.done\n\
data: {\"type\":\"response.output_text.done\",\"sequence_number\":5,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"text\":\"first\"}\n\n\
event: response.content_part.done\n\
data: {\"type\":\"response.content_part.done\",\"sequence_number\":6,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"output_text\",\"text\":\"first\",\"annotations\":[]}}\n\n\
event: response.content_part.added\n\
data: {\"type\":\"response.content_part.added\",\"sequence_number\":7,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":1,\"part\":{\"type\":\"output_text\",\"text\":\"\",\"annotations\":[]}}\n\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"sequence_number\":8,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":1,\"delta\":\"second\"}\n\n\
event: response.output_text.done\n\
data: {\"type\":\"response.output_text.done\",\"sequence_number\":9,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":1,\"text\":\"second\"}\n\n\
event: response.content_part.done\n\
data: {\"type\":\"response.content_part.done\",\"sequence_number\":10,\"item_id\":\"msg_1\",\"output_index\":0,\"content_index\":1,\"part\":{\"type\":\"output_text\",\"text\":\"second\",\"annotations\":[]}}\n\n\
event: response.output_item.done\n\
data: {\"type\":\"response.output_item.done\",\"sequence_number\":11,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_1\",\"role\":\"assistant\",\"status\":\"completed\",\"content\":[{\"type\":\"output_text\",\"text\":\"first\",\"annotations\":[]},{\"type\":\"output_text\",\"text\":\"second\",\"annotations\":[]}]}}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":12,\"response\":{\"id\":\"resp_multi\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n",
    )
    .await;

    assert_eq!(body.matches("event: content_block_start").count(), 2);
    assert_eq!(body.matches("event: content_block_stop").count(), 2);
    assert!(body.contains("\"text\":\"first\""));
    assert!(body.contains("\"text\":\"second\""));
    assert!(!body.contains("stream translation error"));
}

#[tokio::test]
async fn translates_responses_refusal_to_text_block_and_refusal_stop_reason() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"message\",\"id\":\"msg_refusal\",\"role\":\"assistant\",\"status\":\"in_progress\",\"content\":[]}}\n\n\
event: response.content_part.added\n\
data: {\"type\":\"response.content_part.added\",\"sequence_number\":3,\"item_id\":\"msg_refusal\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"refusal\",\"refusal\":\"\"}}\n\n\
event: response.refusal.delta\n\
data: {\"type\":\"response.refusal.delta\",\"sequence_number\":4,\"item_id\":\"msg_refusal\",\"output_index\":0,\"content_index\":0,\"delta\":\"cannot comply\"}\n\n\
event: response.refusal.done\n\
data: {\"type\":\"response.refusal.done\",\"sequence_number\":5,\"item_id\":\"msg_refusal\",\"output_index\":0,\"content_index\":0,\"refusal\":\"cannot comply\"}\n\n\
event: response.content_part.done\n\
data: {\"type\":\"response.content_part.done\",\"sequence_number\":6,\"item_id\":\"msg_refusal\",\"output_index\":0,\"content_index\":0,\"part\":{\"type\":\"refusal\",\"refusal\":\"cannot comply\"}}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":7,\"response\":{\"id\":\"resp_refusal\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n",
    )
    .await;

    assert!(body.contains("\"type\":\"text_delta\""));
    assert!(body.contains("\"text\":\"cannot comply\""));
    assert!(body.contains("\"stop_reason\":\"refusal\""));
    assert!(!body.contains("stream translation error"));
}

#[tokio::test]
async fn uses_function_arguments_done_to_fill_missing_argument_deltas() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_function\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"function_call\",\"id\":\"fc_1\",\"call_id\":\"call_1\",\"name\":\"lookup\",\"arguments\":\"\",\"status\":\"in_progress\"}}\n\n\
event: response.function_call_arguments.done\n\
data: {\"type\":\"response.function_call_arguments.done\",\"sequence_number\":3,\"item_id\":\"fc_1\",\"output_index\":0,\"name\":\"lookup\",\"arguments\":\"{\\\"id\\\":\\\"42\\\"}\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_function\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n",
    )
    .await;

    assert!(body.contains("\"type\":\"tool_use\""));
    assert!(body.contains("\"type\":\"input_json_delta\""));
    assert!(body.contains("id"));
    assert!(body.contains("42"));
    assert!(!body.contains("stream translation error"));
}

#[tokio::test]
async fn translates_custom_tool_input_to_json_string_and_closes_tool_block() {
    let body = translate_body(
        "event: response.created\n\
data: {\"type\":\"response.created\",\"sequence_number\":1,\"response\":{\"id\":\"resp_custom\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"in_progress\"}}\n\n\
event: response.output_item.added\n\
data: {\"type\":\"response.output_item.added\",\"sequence_number\":2,\"output_index\":0,\"item\":{\"type\":\"custom_tool_call\",\"id\":\"ct_1\",\"call_id\":\"call_custom\",\"name\":\"shell\",\"input\":\"\"}}\n\n\
event: response.custom_tool_call_input.delta\n\
data: {\"type\":\"response.custom_tool_call_input.delta\",\"sequence_number\":3,\"output_index\":0,\"item_id\":\"ct_1\",\"delta\":\"pwd\"}\n\n\
event: response.custom_tool_call_input.done\n\
data: {\"type\":\"response.custom_tool_call_input.done\",\"sequence_number\":4,\"output_index\":0,\"item_id\":\"ct_1\",\"input\":\"pwd\"}\n\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":5,\"response\":{\"id\":\"resp_custom\",\"object\":\"response\",\"created_at\":0,\"model\":\"glm-5.1\",\"output\":[],\"status\":\"completed\"}}\n\n",
    )
    .await;

    assert!(body.contains("\"type\":\"tool_use\""));
    assert!(body.contains("\"partial_json\":\"\\\"pwd\\\"\""));
    assert!(body.contains("event: content_block_stop"));
    assert!(body.contains("event: message_stop"));
    assert!(!body.contains("stream translation error"));
}
