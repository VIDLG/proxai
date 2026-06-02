use serde_json::json;

use crate::protocol::anthropic::messages::MessageCreateParamsBase;

use super::translate_request_payload;

#[test]
fn translates_chat_request_to_anthropic_messages_shape() {
    let payload = json!({
        "model": "glm-5.1",
        "messages": [
            {"role": "system", "content": "You are concise."},
            {"role": "developer", "content": [{"type": "text", "text": "Prefer exact answers."}]},
            {"role": "user", "content": [{"type": "text", "text": "Call the tool."}]},
            {
                "role": "assistant",
                "content": "Sure.",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
                }]
            },
            {"role": "tool", "tool_call_id": "call_1", "content": "found"}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "lookup",
                "description": "Lookup a value",
                "parameters": {"properties": {"query": {"type": "string"}}}
            }
        }],
        "tool_choice": {"type": "function", "function": {"name": "lookup"}},
        "stream": false,
        "max_completion_tokens": 128,
        "temperature": 0.2,
        "top_p": 0.9
    });

    let translated = translate_request_payload(&payload, "glm-5.1", "claude-test").unwrap();
    serde_json::from_value::<MessageCreateParamsBase>(translated.clone())
        .expect("translated payload must match Anthropic Messages request schema");

    assert_eq!(translated["model"], "claude-test");
    assert_eq!(translated["max_tokens"], 128);
    assert_eq!(
        translated["system"],
        "You are concise.\n\nPrefer exact answers."
    );
    assert_eq!(translated["stream"], false);
    assert!((translated["temperature"].as_f64().unwrap() - 0.2).abs() < 0.000001);
    assert!((translated["top_p"].as_f64().unwrap() - 0.9).abs() < 0.000001);
    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Call the tool."
        })
    );
    assert_eq!(
        translated["messages"][1]["content"][1],
        json!({
            "type": "tool_use",
            "id": "call_1",
            "name": "lookup",
            "input": {"query": "proxai"}
        })
    );
    assert_eq!(
        translated["messages"][2]["content"][0],
        json!({
            "type": "tool_result",
            "tool_use_id": "call_1",
            "content": "found",
            "is_error": false
        })
    );
    assert_eq!(translated["tools"][0]["type"], "custom");
    assert_eq!(translated["tools"][0]["name"], "lookup");
    assert_eq!(
        translated["tools"][0]["input_schema"],
        json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": []
        })
    );
    assert_eq!(
        translated["tool_choice"],
        json!({"type": "tool", "name": "lookup"})
    );
}
