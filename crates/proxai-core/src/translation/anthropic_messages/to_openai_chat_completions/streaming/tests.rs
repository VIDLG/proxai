use serde_json::json;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::Translator;
use crate::translation::anthropic_messages::continuation::{Continuation, ContinuationEnvelope};
use crate::translation::test_support::translate_sse_fixture;

async fn translate_streaming_response(stream: &str) -> String {
    translate_sse_fixture(
        stream,
        Translator::new(
            RequestProtocol::OpenaiChatCompletions,
            ProviderProtocol::AnthropicMessages,
        ),
    )
    .await
}

#[tokio::test]
async fn translates_empty_context_limit_stream_to_chat_length() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_context_limit\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":0,\"output_tokens\":0}}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"model_context_window_exceeded\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":0,\"output_tokens\":0}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(chunks[0]["choices"][0]["delta"]["role"], "assistant");
    assert_eq!(chunks[1]["choices"][0]["finish_reason"], "length");
    assert_eq!(chunks[2]["choices"], json!([]));
    assert!(body.contains("data: [DONE]"));
    assert!(!body.contains("stream translation error"), "{body}");
}

#[tokio::test]
async fn translates_anthropic_stream_to_chat_completion_sse() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    let chunks = chat_stream_payloads(&body);

    assert!(body.contains("event: message"));
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");
    assert_eq!(chunks[0]["choices"][0]["delta"]["role"], "assistant");
    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "hello");
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 1,
            "total_tokens": 3
        })
    );
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_anthropic_stream_refusal_details_to_chat_refusal_delta() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_refusal_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"refusal\",\"stop_sequence\":null,\"stop_details\":{\"type\":\"refusal\",\"category\":\"cyber\",\"explanation\":\"I can't help with cyber abuse.\"},\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(
        chunks[1]["choices"][0]["finish_reason"],
        serde_json::Value::Null
    );
    assert_eq!(
        chunks[1]["choices"][0]["delta"]["refusal"],
        "I can't help with cyber abuse."
    );
    assert!(chunks[2]["choices"][0]["delta"]["content"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["refusal"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["role"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["tool_calls"].is_null());
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 0,
            "total_tokens": 2
        })
    );
}

#[tokio::test]
async fn translates_anthropic_stream_with_only_thinking_blocks() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_thinking_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"hidden\",\"signature\":\"sig\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\" more\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig2\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(
        chunks[1]["choices"][0]["delta"]["reasoning_content"],
        "hidden"
    );
    assert_eq!(
        chunks[2]["choices"][0]["delta"]["reasoning_content"],
        " more"
    );
    assert_eq!(chunks[4]["choices"][0]["finish_reason"], "stop");
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn streams_signature_only_thinking_as_continuation() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_signature_only\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    let (visible_reasoning, continuation) = ContinuationEnvelope::split_chat_reasoning_content(
        chunks[1]["choices"][0]["delta"]["reasoning_content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(visible_reasoning, "");
    assert_eq!(
        continuation,
        Some(ContinuationEnvelope::from(vec![Continuation::Thinking {
            thinking: "".to_string(),
            signature: "sig".to_string(),
        }])),
    );
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_anthropic_thinking_blocks_before_text() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_thinking_then_text\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"hidden\",\"signature\":\"sig\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\" more\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(
        chunks[1]["choices"][0]["delta"]["reasoning_content"],
        "hidden"
    );
    assert_eq!(
        chunks[2]["choices"][0]["delta"]["reasoning_content"],
        " more"
    );
    assert_eq!(chunks[3]["choices"][0]["delta"]["content"], "hello");
    let (thinking, continuations) = ContinuationEnvelope::split_chat_reasoning_content(
        chunks[4]["choices"][0]["delta"]["reasoning_content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(thinking, "");
    assert_eq!(
        continuations,
        Some(ContinuationEnvelope::from(vec![Continuation::Thinking {
            thinking: "hidden more".to_string(),
            signature: "sig".to_string(),
        }])),
    );
    assert_eq!(chunks[5]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[6]["usage"]["completion_tokens"], 1);
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn maps_anthropic_tool_block_indexes_to_chat_tool_call_indexes() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_tool_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"intro\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"lookup\",\"input\":{}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\":\\\"proxai\\\"}\"}}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(
        chunks[2]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
    assert_eq!(
        chunks[3]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
}

#[tokio::test]
async fn rejects_anthropic_stream_eof_after_terminal_delta_before_message_stop() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_no_stop\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    assert!(body.contains("after terminal message_delta but before message_stop"));
}

#[tokio::test]
async fn rejects_anthropic_stream_eof_before_terminal_delta() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_no_delta\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    assert!(body.contains("before terminal message_delta"));
}

#[tokio::test]
async fn rejects_anthropic_message_stop_before_terminal_delta() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stop_early\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    assert!(body.contains("message_stop before terminal message_delta"));
}

#[tokio::test]
async fn rejects_anthropic_event_after_terminal_delta_before_message_stop() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_after_delta\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"late\"}}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    assert!(body.contains("after terminal message_delta before message_stop"));
}

#[tokio::test]
async fn rejects_anthropic_event_after_message_stop() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_after_stop\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"late\"}}\n\n"
    );
    let body = translate_streaming_response(stream).await;

    assert!(body.contains("semantic event after message_stop"));
}

#[tokio::test]
async fn skips_unrepresentable_anthropic_content_blocks() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_ignored_block\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"server_tool_use\",\"id\":\"srvtoolu_1\",\"caller\":{\"type\":\"direct\"},\"input\":{},\"name\":\"web_search\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "hello");
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
}

#[tokio::test]
async fn skips_anthropic_citation_deltas() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_citation_delta\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"citations\":null,\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"citations_delta\",\"citation\":{\"type\":\"char_location\",\"cited_text\":\"hello\",\"document_index\":0,\"document_title\":null,\"end_char_index\":5,\"file_id\":null,\"start_char_index\":0}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: content_block_stop\n",
        "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"output_tokens_details\":null,\"server_tool_use\":null,\"service_tier\":null,\"input_tokens\":2,\"output_tokens\":1}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let body = translate_streaming_response(stream).await;
    let chunks = chat_stream_payloads(&body);

    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "hello");
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
}

fn chat_stream_payloads(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}
