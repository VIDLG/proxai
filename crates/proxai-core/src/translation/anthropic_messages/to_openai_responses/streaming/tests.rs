use crate::protocol::openai::responses::ResponseStreamEvent;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::Translator;
use crate::translation::test_support::{parse_rendered_events, translate_sse_fixture};

async fn translate_streaming_response(stream: &str) -> String {
    translate_sse_fixture(
        stream,
        Translator::new(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::AnthropicMessages,
        ),
    )
    .await
}

fn assert_openai_response_stream_events_deserialize(body: &str) {
    let events = parse_rendered_events(body);
    assert!(
        !events.is_empty(),
        "translated stream should contain SSE events"
    );
    for event in events {
        let payload = event.data;
        let _: ResponseStreamEvent = serde_json::from_value(payload.clone()).unwrap_or_else(|error| {
            panic!("translated event should deserialize as OpenAI Responses stream event: {error}; payload={payload}")
        });
    }
}

#[tokio::test]
async fn translates_anthropic_message_stream_to_openai_responses_sse() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":2}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

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
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_minimax\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"MiniMax-M2.7-highspeed\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":2}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.reasoning_text.delta"));
    assert!(body.contains("\"delta\":\"plan\""));
    assert!(body.contains("event: response.reasoning_text.done"));
    assert!(body.contains("event: response.completed"));
    assert!(body.contains("proxai:anthropic:v1:"));
    assert!(
        !body.contains("event: error"),
        "thinking block stream must not fail translation: {body}"
    );
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn translates_provider_tool_stream_with_required_nullable_normalization() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_tool\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"lookup\",\"input\":{}}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"q\\\":\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"proxai\\\"}\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":2,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":{\"web_search_requests\":1,\"web_fetch_requests\":0}}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.output_item.added"));
    assert!(body.contains("\"type\":\"function_call\""));
    assert!(body.contains("event: response.function_call_arguments.done"));
    assert!(body.contains("event: response.completed"));
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn translates_interrupted_thinking_then_tool_start_stream_to_parseable_events() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_mimo\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"mimo-v2.5-pro\",\"content\":[],\"container\":null,\"stop_details\":null,\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"write_file\",\"input\":{}}}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: response.created"));
    assert!(body.contains("event: response.reasoning_text.delta"));
    assert!(body.contains("event: response.reasoning_text.done"));
    assert!(
        body.contains("event: response.output_item.added"),
        "body={body}"
    );
    assert!(body.contains("\"type\":\"function_call\""));
    assert!(
        !body.contains("event: response.completed"),
        "interrupted upstream stream should not be translated as completed"
    );
    assert!(
        body.contains("event: error"),
        "strict streaming should report interrupted upstream stream: {body}"
    );
}

#[tokio::test]
async fn translates_max_tokens_stream_to_response_incomplete() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_incomplete\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"partial\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":2}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: response.incomplete"));
    assert!(body.contains("\"status\":\"incomplete\""));
    assert!(body.contains("\"reason\":\"max_output_tokens\""));
    assert!(!body.contains("event: response.completed"));
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn allocates_unique_item_ids_for_multiple_text_and_thinking_blocks() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_multi\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"container\":null,\"stop_details\":null,\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"a\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"b\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":1}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"c\",\"signature\":\"\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":2}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":3,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"d\",\"signature\":\"\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":3}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":4}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("\"id\":\"msg_resp_msg_multi\""));
    assert!(body.contains("\"id\":\"msg_resp_msg_multi_1\""));
    assert!(body.contains("\"item_id\":\"rs_resp_msg_multi\""));
    assert!(body.contains("\"item_id\":\"rs_resp_msg_multi_1\""));
    assert_openai_response_stream_events_deserialize(&body);
}

#[tokio::test]
async fn rejects_unopened_content_block_delta() {
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_bad\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"container\":null,\"stop_details\":null,\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"oops\"}}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: error"));
    assert!(body.contains("unopened content block index 0"));
}

#[tokio::test]
async fn translates_glm_5_2_anthropic_provider_stream_to_openai_responses_sse() {
    // GLM-5.2 smoke test for the responses -> anthropic_messages streaming
    // direction: inbound is OpenAI Responses, provider is Anthropic Messages.
    // The provider emits Anthropic SSE; the proxy must translate it back to
    // OpenAI Responses SSE for the inbound side. Exercises text + thinking +
    // tool_use blocks in a single stream.
    let stream = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_glm52\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.2\",\"content\":[],\"container\":null,\"stop_details\":null,\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":12,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig_glm52\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":1}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":12,\"output_tokens\":5}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

    let body = translate_streaming_response(stream).await;

    assert!(body.contains("event: response.created"), "body={body}");
    assert!(body.contains("\"model\":\"glm-5.2\""), "body={body}");
    assert!(
        body.contains("event: response.reasoning_text.delta"),
        "body={body}"
    );
    assert!(body.contains("\"delta\":\"plan\""), "body={body}");
    assert!(
        body.contains("event: response.reasoning_text.done"),
        "body={body}"
    );
    assert!(
        body.contains("event: response.output_text.delta"),
        "body={body}"
    );
    assert!(
        body.contains("event: response.output_text.done"),
        "body={body}"
    );
    assert!(body.contains("event: response.completed"), "body={body}");
    assert!(
        !body.contains("event: error"),
        "glm-5.2 stream must not fail translation: {body}"
    );
    assert_openai_response_stream_events_deserialize(&body);
}
