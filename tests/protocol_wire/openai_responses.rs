use proxai::protocol::openai_responses::{
    ReasoningEffort, RequestProjection, Response, ResponseCreateParams, Status,
};
use serde_json::json;

#[test]
fn deserializes_responses_request_wire_shape_into_local_protocol_type() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Be concise.",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "Search for proxai."}]
            }
        ],
        "tools": [{
            "type": "function",
            "name": "lookup",
            "parameters": {"type": "object", "properties": {}}
        }],
        "tool_choice": "auto",
        "stream": false,
        "max_output_tokens": 128
    });

    let request = serde_json::from_value::<ResponseCreateParams>(payload)
        .expect("local protocol type should parse Responses wire request");

    assert_eq!(request.model.as_deref(), Some("gpt-5.5"));
    assert!(request.input.is_some());
    assert_eq!(request.tools.as_ref().map(Vec::len), Some(1));
    assert!(request.tool_choice.is_some());
}

#[test]
fn projects_basic_responses_request_from_wire_shape() {
    let payload = json!({
        "model": "gpt-5.5",
        "instructions": "Be concise.",
        "input": "Hello",
        "stream": true,
        "max_output_tokens": 128,
        "temperature": 0.2,
        "reasoning": {"effort": "high"}
    });

    let projection = project_payload(payload);

    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(projection.instructions.as_deref(), Some("Be concise."));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.max_output_tokens, Some(128));
    assert_eq!(projection.temperature, Some(0.2));
    assert_eq!(
        projection.reasoning.and_then(|reasoning| reasoning.effort),
        Some(ReasoningEffort::High)
    );
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

#[test]
fn deserializes_responses_reasoning_effort_as_snake_case() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "Hello",
        "reasoning": {"effort": "high", "summary": "detailed"}
    });

    let request = serde_json::from_value::<ResponseCreateParams>(payload)
        .expect("Responses request should parse snake_case reasoning fields");

    assert_eq!(
        request.reasoning.and_then(|reasoning| reasoning.effort),
        Some(ReasoningEffort::High)
    );
}

#[test]
fn deserializes_responses_response_reasoning_effort_as_snake_case() {
    let payload = json!({
        "id": "resp_123",
        "object": "response",
        "created_at": 0,
        "model": "gpt-5.5",
        "output": [],
        "status": "completed",
        "reasoning": {"effort": "high", "summary": "auto"}
    });

    let response = serde_json::from_value::<Response>(payload)
        .expect("Responses response should parse snake_case reasoning fields");

    assert_eq!(response.status, Status::Completed);
    assert_eq!(
        response.reasoning.and_then(|reasoning| reasoning.effort),
        Some(ReasoningEffort::High)
    );
}

fn project_payload(payload: serde_json::Value) -> RequestProjection {
    RequestProjection::from_payload(&payload).expect("project responses request payload")
}
