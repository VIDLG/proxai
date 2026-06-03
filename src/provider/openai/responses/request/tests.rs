use crate::protocol::openai::responses::{
    IncludeEnum, Reasoning, ReasoningEffort, ReasoningSummary, ResponseTextParam, ServiceTier,
    TextResponseFormatConfiguration, ToolChoiceFunction, ToolChoiceParam, Verbosity,
};
use crate::provider::openai::responses::ToolCategory;

use serde_json::json;

#[test]
fn request_projection_uses_typed_parse_for_standard_response_fields() {
    let payload = json!({
        "model": "gpt-5.5",
        "stream": true,
        "max_output_tokens": 128000,
        "parallel_tool_calls": true,
        "reasoning": {
            "effort": "high",
            "summary": "auto"
        },
        "service_tier": "flex",
        "store": true,
        "include": [
            "message.output_text.logprobs"
        ],
        "tool_choice": {
            "type": "function",
            "name": "edit_file"
        },
        "tools": [
            {
                "type": "function",
                "name": "edit_file"
            },
            {
                "type": "web_search_preview"
            }
        ],
        "text": {
            "verbosity": "low"
        },
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "hello"
                    }
                ]
            }
        ]
    });

    let projection =
        super::projection::project_payload(&payload, None).expect("project request payload");

    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(
        projection.reasoning,
        Some(Reasoning {
            effort: Some(ReasoningEffort::High),
            summary: Some(ReasoningSummary::Auto),
        })
    );
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.max_output_tokens, Some(128000));
    assert_eq!(projection.service_tier, Some(ServiceTier::Flex));
    assert_eq!(projection.parallel_tool_calls, Some(true));
    assert_eq!(projection.store, Some(true));
    assert_eq!(
        projection.include,
        Some(vec![IncludeEnum::MessageOutputTextLogprobs])
    );
    assert_eq!(
        projection.tool_choice,
        Some(ToolChoiceParam::Function(ToolChoiceFunction {
            name: "edit_file".to_string(),
        }))
    );
    assert_eq!(
        projection.text,
        Some(ResponseTextParam {
            format: TextResponseFormatConfiguration::Text,
            verbosity: Some(Verbosity::Low),
        })
    );
}

#[test]
fn request_projection_accepts_multimodal_message_content_for_hint_extraction() {
    let payload = json!({
        "model": "gpt-5.5",
        "stream": true,
        "parallel_tool_calls": true,
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "What is in this image?"
                    },
                    {
                        "type": "input_image",
                        "image_url": "data:image/png;base64,AAAA"
                    }
                ]
            }
        ]
    });

    let projection =
        super::projection::project_payload(&payload, None).expect("project request payload");
    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.parallel_tool_calls, Some(true));
}

#[test]
fn request_projection_accepts_zed_assistant_output_text_history() {
    let payload = json!({
        "model": "gpt-5.5",
        "stream": true,
        "max_output_tokens": 32000,
        "parallel_tool_calls": true,
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "hello"
                    }
                ]
            },
            {
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "output_text",
                        "annotations": [],
                        "text": "Hello! What would you like help with?"
                    }
                ]
            },
            {
                "type": "function_call",
                "call_id": "call_123",
                "name": "list_directory",
                "arguments": "{\"path\":\"proxai\"}"
            },
            {
                "type": "function_call_output",
                "call_id": "call_123",
                "output": "# Folders:\nproxai/src"
            }
        ]
    });

    let projection =
        super::projection::project_payload(&payload, None).expect("project request payload");
    assert_eq!(projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(projection.stream, Some(true));
    assert_eq!(projection.max_output_tokens, Some(32000));
    assert_eq!(projection.parallel_tool_calls, Some(true));
}

#[test]
fn prepare_provider_request_preserves_model_when_route_keeps_it() {
    let payload = json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{ "type": "input_text", "text": "hello" }]
        }]
    });

    let prepared = super::prepare_provider_request(&payload, None, "gpt-5.5", "gpt-5.5").unwrap();

    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap(),
        payload
    );
    assert_eq!(prepared.projection.model.as_deref(), Some("gpt-5.5"));
    assert!(prepared.summary.tool_inventory.is_empty());
}

#[test]
fn prepare_provider_request_rewrites_model_and_builds_summary() {
    let payload = json!({
        "model": "gpt-5.5",
        "tools": [
            {
                "type": "function",
                "name": "shell",
                "description": "Run a command",
                "parameters": { "type": "object", "properties": {} }
            }
        ],
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{ "type": "input_text", "text": "hello" }]
        }]
    });

    let prepared =
        super::prepare_provider_request(&payload, None, "gpt-5.5", "claude-sonnet").unwrap();
    let rewritten = serde_json::from_slice::<serde_json::Value>(&prepared.body).unwrap();

    assert_eq!(
        rewritten.get("model").and_then(serde_json::Value::as_str),
        Some("claude-sonnet")
    );
    assert_eq!(prepared.projection.model.as_deref(), Some("gpt-5.5"));
    assert_eq!(prepared.summary.tool_inventory.len(), 1);
    assert_eq!(
        prepared.summary.tool_inventory[0].category,
        ToolCategory::Function
    );
    assert_eq!(prepared.summary.tool_inventory[0].count, 1);
    assert_eq!(
        prepared.summary.tool_inventory[0].names,
        vec!["shell".to_string()]
    );
}
