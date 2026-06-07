use axum::body::{Body, to_bytes};
use axum::http::{Response, header};
use serde_json::json;

use crate::http_support::into_byte_stream;

use super::{
    translate_non_streaming_payload, translate_request_payload, translate_streaming_stream,
};

#[test]
fn translates_anthropic_request_to_chat_completion_shape() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "stream": true,
        "system": [{"type": "text", "text": "You are concise."}],
        "messages": [
            {
                "role": "user",
                "content": [{"type": "text", "text": "Call the tool."}]
            },
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Sure."},
                    {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {"query": "proxai"}}
                ]
            },
            {
                "role": "user",
                "content": [{"type": "tool_result", "tool_use_id": "toolu_1", "content": "found"}]
            }
        ],
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "description": "Lookup a value",
            "input_schema": {
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }
        }],
        "tool_choice": {"type": "tool", "name": "lookup", "disable_parallel_tool_use": true},
        "temperature": 0.2,
        "top_p": 0.9,
        "stop_sequences": ["END"]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["model"], "claude-sonnet-4-5");
    assert_eq!(translated["max_completion_tokens"], 128);
    assert_eq!(translated["stream"], true);
    assert_eq!(translated["messages"][0]["role"], "system");
    assert_eq!(translated["messages"][0]["content"], "You are concise.");
    assert_eq!(translated["messages"][1]["role"], "user");
    assert_eq!(translated["messages"][1]["content"], "Call the tool.");
    assert_eq!(translated["messages"][2]["role"], "assistant");
    assert_eq!(translated["messages"][2]["content"], "Sure.");
    assert_eq!(
        translated["messages"][2]["tool_calls"][0],
        json!({
            "id": "toolu_1",
            "type": "function",
            "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
        })
    );
    assert_eq!(translated["messages"][3]["role"], "tool");
    assert_eq!(translated["messages"][3]["tool_call_id"], "toolu_1");
    assert_eq!(translated["messages"][3]["content"], "found");
    assert_eq!(translated["tools"][0]["type"], "function");
    assert_eq!(translated["tools"][0]["function"]["name"], "lookup");
    assert_eq!(
        translated["tools"][0]["function"]["parameters"],
        json!({
            "type": "object",
            "properties": {"query": {"type": "string"}},
            "required": ["query"]
        })
    );
    assert_eq!(
        translated["tool_choice"],
        json!({"function": {"name": "lookup"}})
    );
    assert_eq!(translated["parallel_tool_calls"], false);
    assert_eq!(translated["stop"], "END");
}

#[test]
fn splits_mixed_anthropic_user_content_and_tool_result_into_chat_messages() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Here is context."},
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "found"}
            ]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(translated["messages"][0]["content"], "Here is context.");
    assert_eq!(translated["messages"][1]["role"], "tool");
    assert_eq!(translated["messages"][1]["tool_call_id"], "toolu_1");
    assert_eq!(translated["messages"][1]["content"], "found");
}

#[test]
fn translates_anthropic_tool_result_text_blocks_to_chat_tool_message_array() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": [
                    {"type": "text", "text": "found"},
                    {"type": "text", "text": " it"}
                ]
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "tool");
    assert_eq!(translated["messages"][0]["tool_call_id"], "toolu_1");
    assert_eq!(
        translated["messages"][0]["content"],
        json!([
            {"type": "text", "text": "found"},
            {"type": "text", "text": " it"}
        ])
    );
}

#[test]
fn rejects_non_text_anthropic_tool_result_blocks_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "tool_result",
                "tool_use_id": "toolu_1",
                "content": [{
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "iVBORw0KGgo="
                    }
                }]
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool_result content block `image`"));
}

#[test]
fn rejects_anthropic_assistant_text_after_tool_use_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Before."},
                {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "lookup",
                    "input": {"query": "proxai"}
                },
                {"type": "text", "text": "After."}
            ]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("text blocks after tool_use blocks"));
}

#[test]
fn translates_anthropic_output_effort_to_chat_reasoning_effort() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "think"}],
        "output_config": {"effort": "xhigh"},
        "thinking": {"type": "enabled", "budget_tokens": 2048}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning_effort"], "xhigh");
}

#[test]
fn translates_anthropic_thinking_to_chat_reasoning_effort() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "think"}],
        "thinking": {"type": "enabled", "budget_tokens": 9000}
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["reasoning_effort"], "high");
}

#[test]
fn translates_anthropic_container_upload_to_chat_file_part() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{"type": "container_upload", "file_id": "file_123"}]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"],
        json!([{
            "type": "file",
            "file": {"file_data": null, "file_id": "file_123", "filename": null}
        }])
    );
}

#[test]
fn rejects_unsupported_anthropic_tool_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "search"}],
        "tools": [{"type": "web_search_20250305"}]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("Anthropic tool `web_search_20250305`"));
}

#[test]
fn rejects_anthropic_tool_choice_for_missing_chat_tool() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{"role": "user", "content": "Call the tool."}],
        "tools": [{
            "type": "custom",
            "name": "lookup",
            "input_schema": {"type": "object"}
        }],
        "tool_choice": {"type": "tool", "name": "missing"}
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("tool_choice references tool `missing`"));
}

#[test]
fn translates_anthropic_base64_document_to_chat_file_part() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "document",
                "title": "spec.pdf",
                "source": {
                    "type": "base64",
                    "media_type": "application/pdf",
                    "data": "JVBERi0x"
                }
            }]
        }]
    });

    let translated = translate_request_payload(&payload).unwrap();

    assert_eq!(translated["messages"][0]["role"], "user");
    assert_eq!(
        translated["messages"][0]["content"][0],
        json!({
            "type": "file",
            "file": {
                "file_data": "JVBERi0x",
                "file_id": null,
                "filename": "spec.pdf"
            }
        })
    );
}

#[test]
fn rejects_unsupported_anthropic_user_block_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": [{
            "role": "user",
            "content": [{
                "type": "thinking",
                "thinking": "hidden chain of thought",
                "signature": "sig"
            }]
        }]
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();

    assert!(error.contains("user content block `thinking`"));
}

#[test]
fn rejects_anthropic_request_without_messages_for_chat_completion() {
    let payload = json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 128,
        "messages": []
    });

    let error = translate_request_payload(&payload).unwrap_err().to_string();
    assert!(error.contains("must contain at least one message"));
}

#[tokio::test]
async fn translates_anthropic_message_to_chat_completion_shape() {
    let upstream = json!({
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "text", "text": "hello"},
            {"type": "tool_use", "id": "toolu_1", "caller": {"type": "direct"}, "name": "lookup", "input": {"query": "proxai"}}
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 5}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(translated["object"], "chat.completion");
    assert_eq!(translated["id"], "chatcmpl_msg_123");
    assert_eq!(translated["model"], "glm-5.1");
    assert_eq!(translated["choices"][0]["message"]["role"], "assistant");
    assert_eq!(translated["choices"][0]["message"]["content"], "hello");
    assert_eq!(translated["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        translated["choices"][0]["message"]["tool_calls"][0],
        json!({
            "id": "toolu_1",
            "type": "function",
            "function": {"name": "lookup", "arguments": "{\"query\":\"proxai\"}"}
        })
    );
    assert_eq!(
        translated["usage"],
        json!({
            "prompt_tokens": 3,
            "completion_tokens": 5,
            "total_tokens": 8,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        })
    );
}

#[tokio::test]
async fn translates_parallel_anthropic_tool_uses_to_chat_tool_calls() {
    let upstream = json!({
        "id": "msg_parallel_tools",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "text", "text": "I'll check both."},
            {"type": "tool_use", "id": "toolu_weather", "caller": {"type": "direct"}, "name": "weather", "input": {"city": "Shanghai"}},
            {"type": "tool_use", "id": "toolu_news", "caller": {"type": "direct"}, "name": "news", "input": {"topic": "AI"}}
        ],
        "stop_reason": "tool_use",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 4, "output_tokens": 6}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["choices"][0]["message"]["content"],
        "I'll check both."
    );
    assert_eq!(translated["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        translated["choices"][0]["message"]["tool_calls"],
        json!([
            {
                "id": "toolu_weather",
                "type": "function",
                "function": {"name": "weather", "arguments": "{\"city\":\"Shanghai\"}"}
            },
            {
                "id": "toolu_news",
                "type": "function",
                "function": {"name": "news", "arguments": "{\"topic\":\"AI\"}"}
            }
        ])
    );
}

#[tokio::test]
async fn translates_anthropic_web_citations_to_chat_url_annotations() {
    let upstream = json!({
        "id": "msg_cited",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{
            "type": "text",
            "text": "See ProxAI docs for details.",
            "citations": [{
                "type": "web_search_result_location",
                "cited_text": "ProxAI docs",
                "encrypted_index": "idx_1",
                "title": "ProxAI",
                "url": "https://example.com/proxai"
            }]
        }],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["choices"][0]["message"]["annotations"][0],
        json!({
            "type": "url_citation",
            "url_citation": {
                "start_index": 4,
                "end_index": 15,
                "title": "ProxAI",
                "url": "https://example.com/proxai"
            }
        })
    );
}

#[tokio::test]
async fn skips_unmatched_anthropic_web_citations() {
    let upstream = json!({
        "id": "msg_unmatched_cited",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{
            "type": "text",
            "text": "See ProxAI docs for details.",
            "citations": [{
                "type": "web_search_result_location",
                "cited_text": "missing text",
                "encrypted_index": "idx_1",
                "title": "ProxAI",
                "url": "https://example.com/proxai"
            }]
        }],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert!(translated["choices"][0]["message"]["annotations"].is_null());
}

#[tokio::test]
async fn translates_citation_offsets_across_joined_unicode_text_blocks() {
    let upstream = json!({
        "id": "msg_unicode_cited",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [
            {"type": "text", "text": "前缀🙂"},
            {
                "type": "text",
                "text": "见 ProxAI 文档。",
                "citations": [{
                    "type": "web_search_result_location",
                    "cited_text": "ProxAI",
                    "encrypted_index": "idx_1",
                    "title": "ProxAI",
                    "url": "https://example.com/proxai"
                }]
            }
        ],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["choices"][0]["message"]["content"],
        "前缀🙂见 ProxAI 文档。"
    );
    assert_eq!(
        translated["choices"][0]["message"]["annotations"][0]["url_citation"],
        json!({
            "start_index": 5,
            "end_index": 11,
            "title": "ProxAI",
            "url": "https://example.com/proxai"
        })
    );
}

#[tokio::test]
async fn maps_repeated_cited_text_to_later_occurrences_in_order() {
    let upstream = json!({
        "id": "msg_repeated_cited",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{
            "type": "text",
            "text": "ProxAI docs and ProxAI docs",
            "citations": [
                {
                    "type": "web_search_result_location",
                    "cited_text": "ProxAI docs",
                    "encrypted_index": "idx_1",
                    "title": "First",
                    "url": "https://example.com/first"
                },
                {
                    "type": "web_search_result_location",
                    "cited_text": "ProxAI docs",
                    "encrypted_index": "idx_2",
                    "title": "Second",
                    "url": "https://example.com/second"
                }
            ]
        }],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["choices"][0]["message"]["annotations"][0]["url_citation"],
        json!({
            "start_index": 0,
            "end_index": 11,
            "title": "First",
            "url": "https://example.com/first"
        })
    );
    assert_eq!(
        translated["choices"][0]["message"]["annotations"][1]["url_citation"],
        json!({
            "start_index": 16,
            "end_index": 27,
            "title": "Second",
            "url": "https://example.com/second"
        })
    );
}

#[tokio::test]
async fn translates_anthropic_cache_read_usage_to_chat_prompt_details() {
    let upstream = json!({
        "id": "msg_cached",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "cached"}],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {
            "input_tokens": 10,
            "output_tokens": 2,
            "cache_read_input_tokens": 7
        }
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(
        translated["usage"]["prompt_tokens_details"],
        json!({"audio_tokens": null, "cached_tokens": 7})
    );
    assert!(translated["usage"]["completion_tokens_details"].is_null());
}

#[tokio::test]
async fn prefers_refusal_content_over_refusal_details_explanation() {
    let upstream = json!({
        "id": "msg_refusal_details",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "I can't help with that."}],
        "stop_reason": "refusal",
        "stop_details": {
            "type": "refusal",
            "category": "cyber",
            "explanation": "I can't help with cyber abuse."
        },
        "stop_sequence": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(translated["choices"][0]["finish_reason"], "stop");
    assert!(translated["choices"][0]["message"]["content"].is_null());
    assert_eq!(
        translated["choices"][0]["message"]["refusal"],
        "I can't help with that."
    );
}

#[tokio::test]
async fn uses_refusal_details_explanation_when_refusal_content_is_empty() {
    let upstream = json!({
        "id": "msg_refusal_explanation",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [],
        "stop_reason": "refusal",
        "stop_details": {
            "type": "refusal",
            "category": "cyber",
            "explanation": "I can't help with cyber abuse."
        },
        "stop_sequence": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 0}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert!(translated["choices"][0]["message"]["content"].is_null());
    assert_eq!(
        translated["choices"][0]["message"]["refusal"],
        "I can't help with cyber abuse."
    );
}

#[tokio::test]
async fn uses_refusal_text_when_refusal_details_have_no_explanation() {
    let upstream = json!({
        "id": "msg_refusal_text",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "I can't help with that."}],
        "stop_reason": "refusal",
        "stop_details": {
            "type": "refusal",
            "category": "cyber",
            "explanation": null
        },
        "stop_sequence": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert!(translated["choices"][0]["message"]["content"].is_null());
    assert_eq!(
        translated["choices"][0]["message"]["refusal"],
        "I can't help with that."
    );
}

#[tokio::test]
async fn translates_anthropic_refusal_stop_reason_to_chat_stop() {
    let upstream = json!({
        "id": "msg_refusal",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "text", "text": "I can't help with that."}],
        "stop_reason": "refusal",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });
    let translated = translate_non_streaming_payload(upstream).unwrap();

    assert_eq!(translated["choices"][0]["finish_reason"], "stop");
    assert!(translated["choices"][0]["message"]["tool_calls"].is_null());
}

#[tokio::test]
async fn translates_anthropic_stream_to_chat_completion_sse() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello\"}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":2,\"output_tokens\":1,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();

    let chunks = chat_stream_payloads(body);

    assert!(body.contains("event: message"));
    assert_eq!(chunks[0]["object"], "chat.completion.chunk");
    assert_eq!(chunks[0]["choices"][0]["delta"]["role"], "assistant");
    assert_eq!(chunks[1]["choices"][0]["delta"]["content"], "hello");
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 1,
            "total_tokens": 3,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        })
    );
    assert!(body.contains("data: [DONE]"));
}

#[tokio::test]
async fn translates_anthropic_stream_refusal_details_to_chat_refusal_delta() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_refusal_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"refusal\",\"stop_sequence\":null,\"stop_details\":{\"type\":\"refusal\",\"category\":\"cyber\",\"explanation\":\"I can't help with cyber abuse.\"},\"container\":null},\"usage\":{\"input_tokens\":2,\"output_tokens\":0,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    let chunks = chat_stream_payloads(body);

    assert_eq!(
        chunks[1]["choices"][0]["finish_reason"],
        serde_json::Value::Null
    );
    assert_eq!(
        chunks[1]["choices"][0]["delta"]["refusal"],
        "I can't help with cyber abuse."
    );
    assert!(chunks[2]["choices"][0]["delta"]["content"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["refusal"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["role"].is_null());
    assert!(chunks[2]["choices"][0]["delta"]["tool_calls"].is_null());
    assert_eq!(chunks[2]["choices"][0]["finish_reason"], "stop");
    assert_eq!(chunks[3]["choices"], json!([]));
    assert_eq!(
        chunks[3]["usage"],
        json!({
            "prompt_tokens": 2,
            "completion_tokens": 0,
            "total_tokens": 2,
            "prompt_tokens_details": null,
            "completion_tokens_details": null
        })
    );
}

#[tokio::test]
async fn maps_anthropic_tool_block_indexes_to_chat_tool_call_indexes() {
    let stream = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_tool_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":2,\"output_tokens\":0}}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"intro\"}}\n\n",
        "event: content_block_start\n",
        "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"caller\":{\"type\":\"direct\"},\"name\":\"lookup\",\"input\":{}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"query\\\":\\\"proxai\\\"}\"}}\n\n"
    );
    let mut response = Response::new(Body::from(stream));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let response =
        translate_streaming_stream(into_byte_stream(response.into_body().into_data_stream()));
    let body = to_bytes(Body::from_stream(response), usize::MAX)
        .await
        .unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    let chunks = chat_stream_payloads(body);

    assert_eq!(
        chunks[2]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
    assert_eq!(
        chunks[3]["choices"][0]["delta"]["tool_calls"][0]["index"],
        0
    );
}

fn chat_stream_payloads(body: &str) -> Vec<serde_json::Value> {
    body.lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| *data != "[DONE]")
        .map(|data| serde_json::from_str(data).unwrap())
        .collect()
}
