use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionRequest;
use crate::protocol::openai::responses::CreateResponseRequest;

use super::super::translate_request_payload as translate_request_payload_with_translator;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::TranslationResult;
use crate::translation::test_support::request_scope;

fn translate_request_payload(payload: &serde_json::Value) -> TranslationResult<serde_json::Value> {
    let scope = request_scope(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    );
    translate_request_payload_with_translator(payload, &scope)
}

#[test]
fn translates_responses_request_to_chat_completions_shape() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_text", "text": "hello"},
                    {"type": "input_image", "image_url": "https://example.test/a.png", "detail": "low"}
                ]
            },
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call_output", "call_id": "call_1", "output": "result"}
        ],
        "max_output_tokens": 128,
        "parallel_tool_calls": false,
        "reasoning": {"effort": "high"},
        "tool_choice": "required",
        "tools": [{
            "type": "function",
            "strict": null,
            "name": "lookup",
            "description": "Look up a record",
            "parameters": {"type": "object", "properties": {"id": {"type": "string"}}}
        }],
        "stream": true,
        "temperature": 1.0,
        "top_p": 0.9
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["max_completion_tokens"], 128);
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["reasoning_effort"], "high");
    assert_eq!(translated["stream"], true);
    // top-level `instructions` is prepended as a developer message in modern Chat.
    assert_eq!(translated["messages"][0]["role"], "developer");
    assert_eq!(translated["messages"][0]["content"], "Be concise.");
    // The user message follows the developer message.
    let user = &translated["messages"][1];
    assert_eq!(user["role"], "user");
    assert_eq!(user["content"][0]["type"], "text");
    assert_eq!(user["content"][1]["type"], "image_url");
    // Function call maps to an assistant tool_calls message.
    assert_eq!(translated["messages"][2]["role"], "assistant");
    assert_eq!(
        translated["messages"][2]["tool_calls"][0]["type"],
        "function"
    );
    // Function call output maps to a tool message.
    assert_eq!(translated["messages"][3]["role"], "tool");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tool_choice"], "required");
}

#[test]
fn preserves_max_reasoning_effort_for_chat_completions() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": "think deeply",
        "reasoning": {"effort": "max"}
    }))
    .unwrap();

    assert_eq!(translated["reasoning_effort"], "max");
}

#[test]
fn translates_default_responses_text_format_for_chat_completions() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [{"type": "message", "role": "user", "content": "hello"}],
        "text": {"format": {"type": "text"}}
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["response_format"], json!({"type": "text"}));
}

#[test]
fn translates_structured_responses_text_format_for_chat_completions() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [{"type": "message", "role": "user", "content": "hello"}],
        "text": {"format": {"type": "json_object"}}
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(
        translated["response_format"],
        json!({"type": "json_object"})
    );
}

#[test]
fn preserves_responses_system_and_developer_roles_as_chat_instruction_messages() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "top-level developer instruction",
        "input": [
            {
                "type": "message",
                "role": "system",
                "content": [{"type": "input_text", "text": "system instruction"}]
            },
            {
                "type": "message",
                "role": "developer",
                "content": [{"type": "input_text", "text": "developer instruction"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["messages"][0]["role"], "developer");
    assert_eq!(
        translated["messages"][0]["content"],
        "top-level developer instruction"
    );
    assert_eq!(translated["messages"][1]["role"], "system");
    assert_eq!(translated["messages"][1]["content"], "system instruction");
    assert_eq!(translated["messages"][2]["role"], "developer");
    assert_eq!(
        translated["messages"][2]["content"],
        "developer instruction"
    );
    assert_eq!(translated["messages"][3]["role"], "user");
    assert_eq!(translated["messages"][3]["content"], "hello");
}

#[test]
fn translates_normalized_assistant_replay_with_output_text() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {
                "type": "message",
                "id": "msg_zed_replay_0",
                "role": "assistant",
                "status": "completed",
                "phase": "final_answer",
                "content": [{"type": "output_text", "text": "previous answer", "annotations": [], "logprobs": []}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "continue"}]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    serde_json::from_value::<CreateChatCompletionRequest>(translated.clone())
        .expect("translated payload must match Chat Completions request schema");

    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(translated["messages"][0]["content"], "previous answer");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(translated["messages"][1]["content"], "continue");
}

#[test]
fn groups_consecutive_assistant_artifacts_into_one_chat_turn() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "instructions": "Keep the translated history intact.",
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": "continue"
            },
            {
                "type": "reasoning",
                "id": "rs_1",
                "summary": [{"type": "summary_text", "text": "visible reasoning"}],
                "status": "completed"
            },
            {
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}",
                "status": "completed"
            },
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "answer",
                    "annotations": [],
                    "logprobs": []
                }],
                "status": "completed"
            }
        ]
    }))
    .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 3);
    assert_eq!(translated["messages"][0]["role"], "developer");
    assert!(translated["messages"][0].get("reasoning_content").is_none());
    let assistant = &translated["messages"][2];
    assert_eq!(assistant["role"], "assistant");
    assert_eq!(assistant["reasoning_content"], "visible reasoning");
    assert_eq!(assistant["content"], "answer");
    assert_eq!(assistant["tool_calls"][0]["function"]["name"], "lookup");
}

#[test]
fn parses_responses_request_into_native_response_create_params() {
    let payload = json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [{"type": "message", "role": "user", "content": "hi"}],
        "max_output_tokens": 16
    });

    serde_json::from_value::<CreateResponseRequest>(payload).unwrap();
}

#[test]
fn rejects_responses_item_reference_without_referenced_content() {
    let error = translate_request_payload(&json!({
        "model": "glm-5.1",
        "input": [
            {"type": "item_reference", "id": "msg_abc"}
        ]
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("item_reference `msg_abc` cannot be translated"));
    assert!(error.contains("referenced item content is not available"));
}

#[test]
fn rejects_responses_request_without_model() {
    let payload = json!({
        "input": [{"type": "message", "role": "user", "content": "hi"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();
    assert!(error.contains("`model`"));
}

#[test]
fn flattens_function_call_arguments_into_assistant_tool_call() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {"type": "function_call", "call_id": "call_1", "name": "lookup", "arguments": "{\"id\":\"42\"}"},
            {"type": "function_call", "call_id": "call_2", "name": "lookup", "arguments": "{}"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    // Two function calls collapse into a single assistant message with two tool_calls.
    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(
        translated["messages"][0]["tool_calls"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn drops_unrepresentable_item_without_injecting_a_message() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [
            {
                "type": "function_call",
                "call_id": "call_1",
                "name": "lookup",
                "arguments": "{}"
            },
            {
                "type": "additional_tools",
                "role": "developer",
                "tools": [{
                    "type": "function",
                    "name": "late_tool",
                    "parameters": {"type": "object"},
                    "strict": null
                }]
            },
            {
                "type": "function_call_output",
                "call_id": "call_1",
                "output": "result"
            }
        ]
    }))
    .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 2);
    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(translated["messages"][1]["role"], "tool");
    assert_eq!(translated["messages"][1]["tool_call_id"], "call_1");
}

#[test]
fn rejects_encrypted_compaction_item() {
    let error = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "compaction",
            "encrypted_content": "provider-encrypted-summary"
        }]
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("compaction item cannot be translated"));
    assert!(error.contains("summary content is encrypted"));
}

#[test]
fn skips_hosted_responses_tools_without_chat_equivalent() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [{"type": "message", "role": "user", "content": "search"}],
        "tools": [
            {"type": "function", "strict": null, "name": "lookup", "parameters": {"type": "object"}},
            {"type": "web_search"}
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let tools = translated["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["type"], "function");
}

#[test]
fn skips_hosted_tool_choice_without_chat_equivalent() {
    let payload = json!({
        "model": "glm-5.1",
        "input": "search",
        "tool_choice": {"type": "file_search"}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert!(translated.get("tool_choice").is_none());
}

#[test]
fn translates_custom_tool_output_content_list_as_tool_content_array() {
    let payload = json!({
        "model": "glm-5.1",
        "input": [
            {
                "type": "custom_tool_call",
                "call_id": "call_1",
                "name": "shell",
                "input": "pwd"
            },
            {
                "type": "custom_tool_call_output",
                "call_id": "call_1",
                "output": [
                    {
                        "type": "input_text",
                        "text": "first",
                        "prompt_cache_breakpoint": {"mode": "explicit"}
                    },
                    {"type": "input_text", "text": "second"}
                ]
            }
        ]
    });

    let translated = translate_request_payload(&payload).unwrap();
    let content = translated["messages"][1]["content"].as_array().unwrap();

    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["text"], "first");
    assert_eq!(
        content[0]["prompt_cache_breakpoint"],
        json!({"mode": "explicit"})
    );
    assert_eq!(content[1]["text"], "second");
}

#[test]
fn keeps_instruction_content_together_and_preserves_prompt_cache_breakpoints() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "message",
            "role": "system",
            "content": [
                {
                    "type": "input_text",
                    "text": "stable prefix",
                    "prompt_cache_breakpoint": {"mode": "explicit"}
                },
                {"type": "input_text", "text": "current instruction"}
            ]
        }]
    }))
    .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 1);
    assert_eq!(translated["messages"][0]["role"], "system");
    assert_eq!(
        translated["messages"][0]["content"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        translated["messages"][0]["content"][0]["prompt_cache_breakpoint"],
        json!({"mode": "explicit"})
    );
}

#[test]
fn preserves_assistant_history_prompt_cache_breakpoint() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [
            {
                "role": "assistant",
                "content": [{
                    "type": "input_text",
                    "text": "cached answer",
                    "prompt_cache_breakpoint": {"mode": "explicit"}
                }]
            },
            {"role": "user", "content": "continue"}
        ]
    }))
    .unwrap();

    assert_eq!(translated["messages"][0]["role"], "assistant");
    assert_eq!(
        translated["messages"][0]["content"][0]["prompt_cache_breakpoint"],
        json!({"mode": "explicit"})
    );
}

#[test]
fn translates_responses_input_file_to_chat_file_content() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_file",
                "filename": "brief.txt",
                "file_data": "data:text/plain;base64,b2s=",
                "prompt_cache_breakpoint": {"mode": "explicit"}
            }]
        }]
    }))
    .unwrap();

    let file = &translated["messages"][0]["content"][0];
    assert_eq!(file["type"], "file");
    assert_eq!(file["file"]["filename"], "brief.txt");
    assert_eq!(file["file"]["file_data"], "data:text/plain;base64,b2s=");
    assert_eq!(file["prompt_cache_breakpoint"], json!({"mode": "explicit"}));
}

#[test]
fn translates_responses_input_image_file_id_to_chat_file_content() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_image",
                "file_id": "file_image_1",
                "prompt_cache_breakpoint": {"mode": "explicit"}
            }]
        }]
    }))
    .unwrap();

    let file = &translated["messages"][0]["content"][0];
    assert_eq!(file["type"], "file");
    assert_eq!(file["file"]["file_id"], "file_image_1");
    assert_eq!(file["prompt_cache_breakpoint"], json!({"mode": "explicit"}));
}

#[test]
fn preserves_responses_original_image_with_auto_detail_fallback() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_image",
                "image_url": "https://example.test/original.png",
                "detail": "original"
            }]
        }]
    }))
    .unwrap();

    let image = &translated["messages"][0]["content"][0];
    assert_eq!(image["type"], "image_url");
    assert_eq!(
        image["image_url"]["url"],
        "https://example.test/original.png"
    );
    assert_eq!(image["image_url"]["detail"], "auto");
}

#[test]
fn preserves_function_output_text_part_prompt_cache_breakpoint() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [{
            "type": "function_call_output",
            "call_id": "call_1",
            "output": [{
                "type": "input_text",
                "text": "cached tool result",
                "prompt_cache_breakpoint": {"mode": "explicit"}
            }]
        }]
    }))
    .unwrap();

    assert_eq!(translated["messages"][0]["role"], "tool");
    assert_eq!(
        translated["messages"][0]["content"][0]["prompt_cache_breakpoint"],
        json!({"mode": "explicit"})
    );
}

#[test]
fn uses_the_same_user_content_projection_for_easy_and_typed_messages() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "input": [
            {
                "role": "user",
                "content": [{"type": "input_text", "text": "easy"}]
            },
            {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "typed"}]
            }
        ]
    }))
    .unwrap();

    assert_eq!(translated["messages"][0]["content"], "easy");
    assert_eq!(translated["messages"][1]["content"], "typed");
}

#[test]
fn omits_user_turn_without_representable_content() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "instructions": "Keep the instruction-only request intact.",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_image", "detail": "auto"}]
        }]
    }))
    .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 1);
    assert_eq!(translated["messages"][0]["role"], "developer");
    assert_eq!(
        translated["messages"][0]["content"],
        "Keep the instruction-only request intact."
    );
}

#[test]
fn preserves_explicit_empty_user_text() {
    let translated = translate_request_payload(&json!({
        "model": "gpt-5.6",
        "instructions": "Preserve the following user turn.",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": ""}]
        }]
    }))
    .unwrap();

    assert_eq!(translated["messages"].as_array().unwrap().len(), 2);
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(translated["messages"][1]["content"], "");
}
