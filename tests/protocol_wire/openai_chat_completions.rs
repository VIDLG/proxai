use proxai::protocol::openai::chat_completions::{
    ChatCompletionRequestMessage, ChatCompletionResponseMessage, ChatCompletionStreamResponseDelta,
    ChatCompletionTools, CreateChatCompletionRequest, RequestProjection,
};
use serde_json::json;

#[test]
fn deserializes_chat_completions_request_wire_shape_into_local_protocol_types() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "system", "content": "Be concise."},
            {"role": "user", "content": [{"type": "text", "text": "Hello"}]},
            {
                "role": "assistant",
                "content": "Calling a tool",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"q\":\"proxai\"}"}
                }]
            },
            {"role": "tool", "tool_call_id": "call_1", "content": "found"}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "lookup",
                "description": "Lookup a value",
                "parameters": {"properties": {"q": {"type": "string"}}}
            }
        }],
        "tool_choice": {"type": "function", "function": {"name": "lookup"}},
        "max_completion_tokens": 128,
        "stream": false
    });

    let request = serde_json::from_value::<CreateChatCompletionRequest>(payload)
        .expect("local protocol type should parse wire-shaped request");

    assert_eq!(request.model, "gpt-4.1");
    assert_eq!(request.messages.len(), 4);
    assert!(matches!(
        request.messages[0],
        ChatCompletionRequestMessage::System(_)
    ));
    assert!(matches!(
        request.tools.as_ref().unwrap()[0],
        ChatCompletionTools::Function(_)
    ));
    assert!(request.tool_choice.is_some());
}

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

#[test]
fn official_chat_wire_types_do_not_serialize_zed_reasoning_extensions() {
    let request_payload = json!({
        "model": "gpt-4.1",
        "messages": [
            {"role": "user", "content": "Think."},
            {
                "role": "assistant",
                "content": "Answer.",
                "reasoning_content": "Hidden reasoning."
            }
        ]
    });
    let request = serde_json::from_value::<CreateChatCompletionRequest>(request_payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();
    assert!(serialized["messages"][1].get("reasoning_content").is_none());

    let response_message = serde_json::from_value::<ChatCompletionResponseMessage>(json!({
        "role": "assistant",
        "content": "Answer.",
        "refusal": null,
        "reasoning_content": "Hidden reasoning."
    }))
    .unwrap();
    assert!(
        serde_json::to_value(response_message)
            .unwrap()
            .get("reasoning_content")
            .is_none()
    );

    let stream_delta = serde_json::from_value::<ChatCompletionStreamResponseDelta>(json!({
        "reasoning_content": "Hidden reasoning."
    }))
    .unwrap();
    assert!(
        serde_json::to_value(stream_delta)
            .unwrap()
            .get("reasoning_content")
            .is_none()
    );
}

#[test]
fn serializes_chat_request_without_none_optional_fields() {
    let payload = json!({
        "model": "gpt-4.1",
        "messages": [{"role": "user", "content": "Hello"}]
    });
    let request = serde_json::from_value::<CreateChatCompletionRequest>(payload.clone()).unwrap();

    assert_eq!(serde_json::to_value(request).unwrap(), payload);
}
