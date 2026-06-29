use serde_json::json;

use crate::protocol::anthropic::messages::Message;
use crate::protocol::openai::responses::Response as OpenaiResponse;

use super::translate_non_streaming_payload;

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
            "service_tier": "priority"
        }
    }))
    .unwrap();

    let translated: OpenaiResponse = (&message).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();
    let _: OpenaiResponse = serde_json::from_value(value.clone())
        .expect("translated response should deserialize as OpenAI Responses");

    assert_eq!(value["id"], "resp_msg_123");
    assert_eq!(value["object"], "response");
    assert_eq!(value["model"], "glm-5.1");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["service_tier"], "priority");
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
fn preserves_interleaved_text_reasoning_and_tool_order() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_ordered",
        "container": null,
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "thinking", "thinking": "plan", "signature": "sig"},
            {"type": "text", "text": "before tool", "citations": null},
            {
                "type": "tool_use",
                "id": "toolu_1",
                "caller": {"type": "direct"},
                "name": "lookup",
                "input": {"q": "proxai"}
            },
            {"type": "text", "text": "after tool", "citations": null}
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
            "server_tool_use": null,
            "service_tier": null
        }
    }))
    .unwrap();

    let translated: OpenaiResponse = (&message).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["output"][0]["type"], "reasoning");
    assert_eq!(value["output"][1]["type"], "message");
    assert_eq!(value["output"][1]["id"], "msg_msg_ordered");
    assert_eq!(value["output"][1]["content"][0]["text"], "before tool");
    assert_eq!(value["output"][2]["type"], "function_call");
    assert_eq!(value["output"][3]["type"], "message");
    assert_eq!(value["output"][3]["id"], "msg_msg_ordered_1");
    assert_eq!(value["output"][3]["content"][0]["text"], "after tool");
}

#[test]
fn translates_anthropic_tool_result_to_function_call_output() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_tool_result",
        "container": null,
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {
                "type": "tool_use",
                "id": "toolu_1",
                "caller": {"type": "direct"},
                "name": "lookup",
                "input": {"q": "proxai"}
            },
            {
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": "found"
            }
        ],
        "stop_details": null,
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 6,
            "cache_creation": null,
            "cache_creation_input_tokens": null,
            "cache_read_input_tokens": null,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": null
        }
    }))
    .unwrap();

    let translated: OpenaiResponse = (&message).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["output"][0]["type"], "function_call");
    assert_eq!(value["output"][1]["type"], "function_call_output");
    assert_eq!(value["output"][1]["id"], "fco_msg_tool_result");
    assert_eq!(value["output"][1]["call_id"], "toolu_1");
    assert_eq!(value["output"][1]["output"], "found");
    assert_eq!(value["output"][1]["status"], "completed");
}

#[test]
fn translates_max_tokens_stop_to_incomplete_details() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_incomplete",
        "container": null,
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "partial", "citations": null}],
        "stop_details": null,
        "stop_reason": "max_tokens",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 6,
            "cache_creation": null,
            "cache_creation_input_tokens": null,
            "cache_read_input_tokens": null,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": "standard"
        }
    }))
    .unwrap();

    let translated: OpenaiResponse = (&message).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["incomplete_details"]["reason"], "max_output_tokens");
    assert_eq!(value["service_tier"], "default");
}

#[test]
fn omits_unrepresentable_anthropic_batch_service_tier() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_batch",
        "container": null,
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "ok", "citations": null}],
        "stop_details": null,
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "usage": {
            "input_tokens": 1,
            "output_tokens": 1,
            "cache_creation": null,
            "cache_creation_input_tokens": null,
            "cache_read_input_tokens": null,
            "inference_geo": null,
            "server_tool_use": null,
            "service_tier": "batch"
        }
    }))
    .unwrap();

    let translated: OpenaiResponse = (&message).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert!(value["service_tier"].is_null());
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

#[test]
fn translates_anthropic_web_citations_to_responses_url_annotations() {
    let upstream = json!({
        "id": "msg_cited",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{
            "type": "text",
            "text": "See ProxAI docs for details.",
            "citations": [{
                "type": "web_search_result_location",
                "cited_text": "ProxAI docs",
                "encrypted_index": "idx_1",
                "title": "ProxAI",
                "url": "https://example.com/proxai"
            }]
        }],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["output"][0]["content"][0]["annotations"][0],
        json!({
            "UrlCitation": {
                "start_index": 4,
                "end_index": 15,
                "title": "ProxAI",
                "url": "https://example.com/proxai"
            }
        })
    );
}
