use serde_json::json;

use super::RequestProjection;

#[test]
fn projection_ignores_unknown_input_items() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            },
            {
                "type": "future_zed_item",
                "opaque": {"value": 1}
            }
        ],
        "include": ["reasoning.encrypted_content"],
        "parallel_tool_calls": true,
        "prompt_cache_key": "zed-session",
        "stream": true,
        "max_output_tokens": 128
    });

    let projection = RequestProjection::from_payload(&payload).unwrap();

    assert_eq!(projection.model.as_deref(), Some("glm-5.1"));
    assert_eq!(projection.max_output_tokens, Some(128));
    assert_eq!(projection.parallel_tool_calls, Some(true));
    assert_eq!(projection.prompt_cache_key.as_deref(), Some("zed-session"));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.include.as_ref().map(Vec::len), Some(1));
}
