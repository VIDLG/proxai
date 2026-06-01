use proxai::protocol::openai::chat_completions::RequestProjection;
use serde_json::json;

#[test]
fn projects_basic_chat_completions_request_from_wire_shape() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "system", "content": "Be concise."},
            {"role": "user", "content": "Hello"}
        ],
        "stream": true,
        "temperature": 0.2,
        "max_completion_tokens": 128
    });

    let projection = RequestProjection::from_payload(&payload)
        .expect("project chat completions request payload");

    assert_eq!(projection.model.as_deref(), Some("gpt-4.1"));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.temperature, Some(0.2));
    assert_eq!(projection.max_completion_tokens, Some(128));
}

#[test]
fn projects_chat_completions_tools_and_response_format_from_wire_shape() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "user", "content": "What is the weather in London?"}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "weather_answer",
                "schema": {
                    "type": "object",
                    "properties": {"answer": {"type": "string"}},
                    "required": ["answer"]
                },
                "strict": true
            }
        },
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather for a city.",
                "parameters": {
                    "type": "object",
                    "properties": {"city": {"type": "string"}},
                    "required": ["city"]
                }
            }
        }],
        "tool_choice": "auto"
    });

    let projection = RequestProjection::from_payload(&payload)
        .expect("project chat completions request with tools");

    assert_eq!(projection.model.as_deref(), Some("gpt-4.1"));
    assert_eq!(projection.tools.as_ref().map(Vec::len), Some(1));
    assert!(projection.tool_choice.is_some());
    assert!(projection.response_format.is_some());
}
