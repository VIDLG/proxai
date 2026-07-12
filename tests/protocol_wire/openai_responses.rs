use proxai::protocol::openai_responses::{
    CreateResponseRequest, ReasoningEffort, RequestProjection, Response, Status,
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
            "parameters": {"type": "object", "properties": {}},
            "strict": null
        }],
        "tool_choice": "auto",
        "stream": false,
        "max_output_tokens": 128
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload)
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
        projection
            .reasoning
            .and_then(|reasoning| reasoning.effort.into_non_null()),
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
                },
                "strict": null
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

    let request = serde_json::from_value::<CreateResponseRequest>(payload)
        .expect("Responses request should parse snake_case reasoning fields");

    assert_eq!(
        request
            .reasoning
            .into_non_null()
            .and_then(|reasoning| reasoning.effort.into_non_null()),
        Some(ReasoningEffort::High)
    );
}

#[test]
fn deserializes_responses_response_reasoning_effort_as_snake_case() {
    let payload = json!({
        "id": "resp_123",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "created_at": 0.0,
        "model": "gpt-5.5",
        "output": [],
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "status": "completed",
        "reasoning": {"effort": "high", "summary": "auto"}
    });

    let response = serde_json::from_value::<Response>(payload.clone())
        .expect("Responses response should parse snake_case reasoning fields");

    assert_eq!(response.status, Some(Status::Completed));
    assert_eq!(
        response
            .reasoning
            .as_non_null()
            .and_then(|reasoning| reasoning.effort.as_non_null())
            .copied(),
        Some(ReasoningEffort::High)
    );
    assert_eq!(serde_json::to_value(response).unwrap(), payload);
}

#[test]
fn preserves_omitted_responses_text_format() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "Hello",
        "text": {"verbosity": "low"}
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert_eq!(serialized["text"], json!({"verbosity": "low"}));
}

#[test]
fn preserves_omitted_easy_input_message_type() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{"role": "user", "content": "Hello"}]
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert!(serialized["input"][0].get("type").is_none());
}

#[test]
fn serializes_responses_request_side_required_nullable_fields_as_null() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "role": "user",
            "content": [
                {"type": "input_image", "image_url": "https://example.test/image.png"},
                {"type": "input_file", "file_url": "https://example.test/spec.pdf"}
            ]
        }],
        "reasoning": {},
        "text": {
            "format": {
                "type": "json_schema",
                "name": "answer",
                "schema": {"type": "object"}
            }
        },
        "tools": [{"type": "function", "name": "lookup", "parameters": null, "strict": null}]
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert_eq!(
        serialized["input"][0]["content"][0],
        json!({
            "type": "input_image",
            "detail": "auto",
            "image_url": "https://example.test/image.png"
        })
    );
    assert_eq!(
        serialized["input"][0]["content"][1],
        json!({
            "type": "input_file",
            "file_url": "https://example.test/spec.pdf"
        })
    );
    assert_eq!(serialized["reasoning"], json!({}));
    assert_eq!(
        serialized["text"]["format"],
        json!({
            "type": "json_schema",
            "name": "answer",
            "schema": {"type": "object"}
        })
    );
    assert_eq!(
        serialized["tools"][0],
        json!({
            "type": "function",
            "name": "lookup",
            "parameters": null,
            "strict": null
        })
    );
}

#[test]
fn serializes_responses_hosted_tools_without_null_fields() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "Hello",
        "tools": [
            {"type": "file_search", "vector_store_ids": ["vs_1"]},
            {"type": "web_search", "user_location": {}},
            {"type": "image_generation"},
            {
                "type": "mcp",
                "server_label": "docs",
                "server_url": "https://example.test/mcp"
            }
        ]
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert_eq!(
        serialized["tools"],
        json!([
            {"type": "file_search", "vector_store_ids": ["vs_1"]},
            {
                "type": "web_search",
                "user_location": {"type": "approximate"}
            },
            {"type": "image_generation"},
            {
                "type": "mcp",
                "server_label": "docs",
                "server_url": "https://example.test/mcp"
            }
        ])
    );
}

#[test]
fn serializes_responses_deferred_and_environment_tools_without_null_fields() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": "Hello",
        "prompt": {"id": "pmpt_1"},
        "tools": [
            {
                "type": "custom",
                "name": "shell_text",
                "description": "Run text commands",
                "format": {"type": "text"}
            },
            {"type": "tool_search"},
            {"type": "shell"},
            {"type": "code_interpreter", "container": {"type": "auto"}}
        ]
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert_eq!(serialized["prompt"], json!({"id": "pmpt_1"}));
    assert_eq!(
        serialized["tools"],
        json!([
            {
                "type": "custom",
                "name": "shell_text",
                "description": "Run text commands",
                "format": {"type": "text"}
            },
            {"type": "tool_search"},
            {"type": "shell"},
            {"type": "code_interpreter", "container": {"type": "auto"}}
        ])
    );
}

#[test]
fn serializes_responses_context_items_with_upstream_optional_field_behavior() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [
            {
                "type": "function_call_output",
                "call_id": "call_function",
                "output": "done"
            },
            {
                "type": "custom_tool_call_output",
                "call_id": "call_custom",
                "output": "done"
            },
            {
                "type": "mcp_approval_response",
                "approval_request_id": "approval_1",
                "approve": true
            },
            {
                "type": "computer_call_output",
                "call_id": "call_computer",
                "output": {"type": "computer_screenshot"}
            },
            {
                "type": "local_shell_call_output",
                "id": "call_local_shell",
                "output": "done"
            },
            {
                "type": "tool_search_call"
            },
            {
                "type": "tool_search_output",
                "tools": []
            },
            {
                "type": "shell_call",
                "call_id": "call_shell",
                "action": {"commands": ["echo done"]}
            },
            {
                "type": "shell_call_output",
                "call_id": "call_shell",
                "output": []
            },
            {
                "type": "apply_patch_call",
                "call_id": "call_patch",
                "status": "in_progress",
                "operation": {"type": "delete_file", "path": "old.txt"}
            },
            {
                "type": "apply_patch_call_output",
                "call_id": "call_patch",
                "status": "completed"
            },
            {
                "type": "compaction",
                "encrypted_content": "encrypted"
            }
        ]
    });

    let request = serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
    let serialized = serde_json::to_value(request).unwrap();

    assert_eq!(
        serialized["input"],
        json!([
            {
                "type": "function_call_output",
                "call_id": "call_function",
                "output": "done"
            },
            {
                "type": "custom_tool_call_output",
                "call_id": "call_custom",
                "output": "done"
            },
            {
                "type": "mcp_approval_response",
                "approval_request_id": "approval_1",
                "approve": true
            },
            {
                "type": "computer_call_output",
                "call_id": "call_computer",
                "output": {"type": "computer_screenshot"}
            },
            {
                "type": "local_shell_call_output",
                "id": "call_local_shell",
                "output": "done"
            },
            {
                "type": "tool_search_call",
                "arguments": null
            },
            {
                "type": "tool_search_output",
                "tools": []
            },
            {
                "type": "shell_call",
                "call_id": "call_shell",
                "action": {"commands": ["echo done"]}
            },
            {
                "type": "shell_call_output",
                "call_id": "call_shell",
                "output": []
            },
            {
                "type": "apply_patch_call",
                "call_id": "call_patch",
                "status": "in_progress",
                "operation": {"type": "delete_file", "path": "old.txt"}
            },
            {
                "type": "apply_patch_call_output",
                "call_id": "call_patch",
                "status": "completed"
            },
            {
                "type": "compaction",
                "encrypted_content": "encrypted"
            }
        ])
    );
}

fn project_payload(payload: serde_json::Value) -> RequestProjection {
    RequestProjection::from_payload(&payload).expect("project responses request payload")
}
