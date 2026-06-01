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
