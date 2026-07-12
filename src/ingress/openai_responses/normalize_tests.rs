use super::*;
use serde_json::json;

#[test]
fn moves_system_input_to_instructions_and_preserves_other_fields() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Existing instructions.",
        "prompt_cache_key": "zed-session",
        "tools": [{"type": "function", "name": "shell"}],
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
        json!([{"type": "function", "name": "shell"}])
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
