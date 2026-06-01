use async_openai::types::responses::CreateResponse;
use proxai::protocol::openai_responses::RequestProjection;
use serde_json::json;

#[test]
fn projects_basic_responses_request_from_wire_shape() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Be concise.",
        "input": "Hello",
        "stream": true,
        "max_output_tokens": 128,
        "temperature": 0.2
    });

    let projection = project_payload(payload);

    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(projection.instructions.as_deref(), Some("Be concise."));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.max_output_tokens, Some(128));
    assert_eq!(projection.temperature, Some(0.2));
}

#[test]
fn projects_responses_tools_and_text_config_from_wire_shape() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Search for proxai."}]
            }
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "answer",
                "schema": {
                    "type": "object",
                    "properties": {"answer": {"type": "string"}},
                    "required": ["answer"]
                },
                "strict": true
            }
        },
        "tools": [
            {"type": "web_search_preview"},
            {
                "type": "function",
                "name": "lookup",
                "parameters": {
                    "type": "object",
                    "properties": {"query": {"type": "string"}},
                    "required": ["query"]
                }
            }
        ],
        "tool_choice": "auto"
    });

    let projection = project_payload(payload);

    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(projection.tools.as_ref().map(Vec::len), Some(2));
    assert!(projection.text.is_some());
    assert!(projection.tool_choice.is_some());
}

fn project_payload(payload: serde_json::Value) -> RequestProjection {
    serde_json::from_value::<CreateResponse>(payload)
        .map(Into::into)
        .expect("project responses request payload")
}
