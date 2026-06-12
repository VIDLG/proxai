use serde_json::json;

use super::translate_non_streaming_payload;

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
async fn rejects_anthropic_response_without_chat_representable_content() {
    let upstream = json!({
        "id": "msg_empty",
        "type": "message",
        "role": "assistant",
        "model": "glm-5.1",
        "content": [{"type": "thinking", "thinking": "hidden", "signature": "sig"}],
        "stop_reason": "end_turn",
        "stop_sequence": null,
        "stop_details": null,
        "container": null,
        "usage": {"input_tokens": 3, "output_tokens": 4}
    });

    let error = translate_non_streaming_payload(upstream)
        .unwrap_err()
        .to_string();

    assert!(error.contains("no Chat-representable content"));
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
