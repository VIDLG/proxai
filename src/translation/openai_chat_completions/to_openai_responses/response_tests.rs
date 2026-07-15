use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionResponse;
use crate::protocol::openai_responses::Response;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::TranslationResult;
use crate::translation::test_support::response_scope;

fn translate_response(chat: &CreateChatCompletionResponse) -> TranslationResult<Response> {
    let scope = response_scope(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    );
    super::translate_response(chat, None, &scope)
}

#[test]
fn translates_chat_completion_response_to_responses_shape() {
    let upstream = json!({
        "id": "chatcmpl_123",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": [{
            "index": 0,
            "message": {
                "refusal": null,
                "role": "assistant",
                "content": "hello",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"id\":\"42\"}"}
                }]
            },
            "finish_reason": "tool_calls",
            "logprobs": null,
            "logprobs": null
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15,
            "completion_tokens_details": {"reasoning_tokens": 2}
        }
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(upstream).unwrap();
    let translated = translate_response(&chat).unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["id"], "resp_chatcmpl_123");
    assert_eq!(value["object"], "response");
    assert_eq!(value["model"], "MiniMax-M3");
    assert_eq!(value["status"], "completed");
    assert_eq!(value["output"][0]["type"], "message");
    assert_eq!(value["output"][0]["content"][0]["type"], "output_text");
    assert_eq!(value["output"][0]["content"][0]["text"], "hello");
    assert_eq!(value["output"][1]["type"], "function_call");
    assert_eq!(value["output"][1]["name"], "lookup");
    assert_eq!(value["usage"]["input_tokens"], 10);
    assert_eq!(
        value["usage"]["output_tokens_details"]["reasoning_tokens"],
        2
    );
}

#[test]
fn translates_zed_non_streaming_reasoning_extension_to_responses_item() {
    let upstream = json!({
        "id": "chatcmpl_reasoning",
        "object": "chat.completion",
        "created": 1234,
        "model": "gpt-test",
        "choices": [{
            "index": 0,
            "message": {
                "refusal": null,
                "role": "assistant",
                "content": "answer",
                "reasoning_content": "thinking"
            },
            "finish_reason": "stop",
            "logprobs": null,
            "logprobs": null
        }]
    });

    let scope = response_scope(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiChatCompletions,
    );
    let translated = super::super::translate_non_streaming_response(upstream, &scope).unwrap();
    assert_eq!(translated["output"][0]["type"], "reasoning");
    assert_eq!(translated["output"][0]["content"][0]["text"], "thinking");
    assert_eq!(translated["output"][1]["type"], "message");
}

#[test]
fn rejects_chat_response_without_choices() {
    let no_choices = json!({
        "id": "chatcmpl_empty_choices",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": []
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(no_choices).unwrap();
    let error = translate_response(&chat).unwrap_err().to_string();
    assert!(error.contains("has no choices"));
}

#[test]
fn rejects_chat_response_with_multiple_choices() {
    let multiple_choices = json!({
        "id": "chatcmpl_multi_choices",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": [
            {
                "index": 0,
                "message": {"content": null, "refusal": null, "role": "assistant", "content": "first"},
                    "content": null,
                    "refusal": null,
                "finish_reason": "stop",
                "logprobs": null,
                "logprobs": null
            },
            {
                "index": 1,
                "message": {"content": null, "refusal": null, "role": "assistant", "content": "second"},
                    "content": null,
                    "refusal": null,
                "finish_reason": "stop",
                "logprobs": null,
                "logprobs": null
            }
        ]
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(multiple_choices).unwrap();
    let error = translate_response(&chat).unwrap_err().to_string();
    assert!(error.contains("target response can represent exactly one assistant message"));
}

#[test]
fn rejects_chat_response_without_responses_output() {
    let upstream = json!({
        "id": "chatcmpl_empty",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": [{
            "index": 0,
            "message": {"content": null, "refusal": null, "role": "assistant", "content": ""},
                "content": null,
                "refusal": null,
            "finish_reason": "stop",
            "logprobs": null,
            "logprobs": null
        }]
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(upstream).unwrap();
    let error = translate_response(&chat).unwrap_err().to_string();

    assert!(error.contains("without content, refusal, or tool calls"));
}
