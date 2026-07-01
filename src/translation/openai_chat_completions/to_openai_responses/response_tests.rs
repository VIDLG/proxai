use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionResponse;
use crate::protocol::openai_responses::Response;

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
                "role": "assistant",
                "content": "hello",
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {"name": "lookup", "arguments": "{\"id\":\"42\"}"}
                }]
            },
            "finish_reason": "tool_calls",
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
    let translated: Response = (&chat).try_into().unwrap();
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
fn rejects_chat_response_without_choices() {
    let no_choices = json!({
        "id": "chatcmpl_empty_choices",
        "object": "chat.completion",
        "created": 1234,
        "model": "MiniMax-M3",
        "choices": []
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(no_choices).unwrap();
    let error = <Response as TryFrom<&CreateChatCompletionResponse>>::try_from(&chat)
        .unwrap_err()
        .to_string();
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
                "message": {"role": "assistant", "content": "first"},
                "finish_reason": "stop",
                "logprobs": null
            },
            {
                "index": 1,
                "message": {"role": "assistant", "content": "second"},
                "finish_reason": "stop",
                "logprobs": null
            }
        ]
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(multiple_choices).unwrap();
    let error = <Response as TryFrom<&CreateChatCompletionResponse>>::try_from(&chat)
        .unwrap_err()
        .to_string();
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
            "message": {"role": "assistant", "content": ""},
            "finish_reason": "stop",
            "logprobs": null
        }]
    });
    let chat = serde_json::from_value::<CreateChatCompletionResponse>(upstream).unwrap();
    let error = <Response as TryFrom<&CreateChatCompletionResponse>>::try_from(&chat)
        .unwrap_err()
        .to_string();

    assert!(error.contains("without content, refusal, or tool calls"));
}
