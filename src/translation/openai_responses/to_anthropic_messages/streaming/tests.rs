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
