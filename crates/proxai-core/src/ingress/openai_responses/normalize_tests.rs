use super::*;
use serde_json::json;

#[test]
fn moves_system_input_to_instructions_and_normalizes_compact_tools() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Existing instructions.",
        "prompt_cache_key": "zed-session",
        "tools": [{
            "type": "function",
            "name": "shell",
            "parameters": {"type": "object", "properties": {}}
        }],
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "System A."}]
            },
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "text", "text": "System B."}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Hello"}]
            }
        ]
    });
    let normalized = normalize_payload(payload);

    assert_eq!(
        normalized["instructions"],
        "System A.\n\nSystem B.\n\nExisting instructions."
    );
    assert_eq!(normalized["input"].as_array().unwrap().len(), 1);
    assert_eq!(normalized["input"][0]["role"], "user");
    assert_eq!(normalized["prompt_cache_key"], "zed-session");
    assert_eq!(
        normalized["tools"],
        json!([{
            "type": "function",
            "name": "shell",
            "parameters": {"type": "object", "properties": {}},
            "strict": null
        }])
    );
}

#[test]
fn does_not_extract_system_input_when_existing_instructions_is_not_string() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": "invalid for request instructions"}]}],
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "System A."}]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert_eq!(normalized["input"].as_array().unwrap().len(), 1);
    assert_eq!(normalized["input"][0]["role"], "system");
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect_err("non-string request instructions should remain a protocol error");
}

#[test]
fn leaves_unextractable_system_input_for_protocol_validation() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "output_text", "text": "not valid system input"}]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert!(normalized.get("instructions").is_none());
    assert_eq!(normalized["input"].as_array().unwrap().len(), 1);
    assert_eq!(normalized["input"][0]["role"], "system");
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect_err("invalid system content should remain visible to protocol parsing");
}

#[test]
fn leaves_mixed_system_input_content_in_input() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [
                    {"type": "input_text", "text": "System A."},
                    {"type": "input_image", "image_url": "https://example.com/image.png"}
                ]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert!(normalized.get("instructions").is_none());
    assert_eq!(normalized["input"].as_array().unwrap().len(), 1);
    assert_eq!(
        normalized["input"][0]["content"].as_array().unwrap().len(),
        2
    );
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("mixed system content should remain in input and parse as Responses");
}

#[test]
fn does_not_normalize_mixed_assistant_replay_content() {
    let payload = json!({
        "model": "glm-5.2",
        "input": [
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "output_text", "text": "previous answer"},
                    {"type": "input_text", "text": "unexpected mixed content"}
                ]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert!(normalized["input"][0].get("id").is_none());
    assert!(normalized["input"][0].get("status").is_none());
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect_err("mixed assistant replay content should not be partially normalized");
}

#[test]
fn normalizes_zed_message_image_detail_to_official_default() {
    let payload = json!({
        "model": "glm-5.2",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_image", "image_url": "data:image/png;base64,c2FuaXRpemVk"},
                    {"type": "input_image", "image_url": "https://example.com/image.png", "detail": "high"}
                ]
            },
            {
                "type": "function_call_output",
                "call_id": "call_1",
                "output": [{
                    "type": "input_image",
                    "image_url": "data:image/png;base64,c2FuaXRpemVk"
                }]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert_eq!(normalized["input"][0]["content"][0]["detail"], "auto");
    assert_eq!(normalized["input"][0]["content"][1]["detail"], "high");
    assert!(normalized["input"][1]["output"][0].get("detail").is_none());
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("normalized Zed message images must match the Responses request schema");
}

#[test]
fn keeps_official_compatible_zed_context_items_unchanged() {
    let payload = json!({
        "model": "glm-5.2",
        "input": [
            {
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}"
            },
            {
                "type": "function_call_output",
                "call_id": "call_1",
                "output": "sanitized result"
            },
            {
                "type": "compaction",
                "encrypted_content": "sanitized-compaction"
            },
            {
                "type": "reasoning",
                "id": "rs_1",
                "summary": []
            }
        ],
        "include": ["reasoning.encrypted_content"],
        "stream": true,
        "store": false,
        "service_tier": "priority",
        "reasoning": {"effort": "max", "summary": "auto"},
        "context_management": [{"type": "compaction", "compact_threshold": 1000}]
    });

    let normalized = normalize_payload(payload.clone());

    assert_eq!(normalized, payload);
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("official-compatible Zed context items must parse without normalization");
}

#[test]
fn does_not_invent_function_parameters_that_zed_builder_always_supplies() {
    let normalized = normalize_payload(json!({
        "model": "glm-5.2",
        "input": "sanitized",
        "tools": [{"type": "function", "name": "lookup"}]
    }));

    assert_eq!(normalized["tools"][0]["strict"], serde_json::Value::Null);
    assert!(normalized["tools"][0].get("parameters").is_none());
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect_err("missing function parameters are not a Zed builder compatibility shape");
}

#[test]
fn normalizes_zed_assistant_replay_to_complete_output_message() {
    let payload = json!({
        "model": "glm-5.2",
        "input": [
            {
                "type": "message",
                "role": "assistant",
                "phase": "final_answer",
                "content": [{"type": "output_text", "text": "previous answer"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "continue"}]
            }
        ]
    });

    let normalized = normalize_payload(payload);

    assert_eq!(normalized["input"][0]["type"], "message");
    assert_eq!(normalized["input"][0]["id"], "msg_zed_replay_0");
    assert_eq!(normalized["input"][0]["status"], "completed");
    assert_eq!(normalized["input"][0]["content"][0]["type"], "output_text");
    assert_eq!(
        normalized["input"][0]["content"][0]["annotations"],
        json!([])
    );
    assert_eq!(normalized["input"][0]["content"][0]["logprobs"], json!([]));
    assert_eq!(normalized["input"][1]["role"], "user");
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("normalized Zed assistant replay must match the native Responses schema");
}

#[test]
fn normalizes_zed_assistant_refusal_replay_to_complete_output_message() {
    let payload = json!({
        "model": "glm-5.2",
        "input": [{
            "role": "assistant",
            "content": [{"type": "refusal", "refusal": "I can't help with that."}]
        }]
    });

    let normalized = normalize_payload(payload);

    assert_eq!(normalized["input"][0]["type"], "message");
    assert_eq!(normalized["input"][0]["id"], "msg_zed_replay_0");
    assert_eq!(normalized["input"][0]["status"], "completed");
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("normalized Zed refusal replay must match the native Responses schema");
}

#[test]
fn preserves_zed_assistant_replay_annotations_and_logprobs() {
    let annotations = json!([{
        "type": "url_citation",
        "url": "https://example.com",
        "start_index": 0,
        "end_index": 8,
        "title": "Example"
    }]);
    let logprobs = json!([{
        "token": "previous",
        "logprob": -0.5,
        "bytes": [112],
        "top_logprobs": []
    }]);
    let payload = json!({
        "model": "glm-5.2",
        "input": [{
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": "previous answer",
                "annotations": annotations,
                "logprobs": logprobs
            }]
        }]
    });

    let normalized = normalize_payload(payload);

    assert_eq!(
        normalized["input"][0]["content"][0]["annotations"],
        annotations
    );
    assert_eq!(normalized["input"][0]["content"][0]["logprobs"], logprobs);
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(normalized)
        .expect("Zed replay with native output metadata must remain valid");
}

#[test]
fn preserves_explicit_function_tool_strictness_and_non_function_tools() {
    let normalized = normalize_payload(json!({
        "tools": [
            {"type": "function", "name": "loose", "strict": false},
            {"type": "function", "name": "unspecified", "strict": null},
            {"type": "web_search_preview"}
        ]
    }));

    assert_eq!(normalized["tools"][0]["strict"], false);
    assert_eq!(normalized["tools"][1]["strict"], serde_json::Value::Null);
    assert!(normalized["tools"][2].get("strict").is_none());
}
