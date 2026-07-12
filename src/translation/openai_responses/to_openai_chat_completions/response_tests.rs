use serde_json::json;

use crate::protocol::openai::chat_completions::CreateChatCompletionResponse;
use crate::protocol::openai::responses::Response;

#[test]
fn translates_responses_message_response_to_chat_completion() {
    let upstream = json!({
        "id": "resp_42",
        "model": "glm-5.1",
        "created_at": 1700000000,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {
                "type": "message",
                "id": "msg_42",
                "role": "assistant",
                "status": "completed",
                "content": [
                    {"type": "output_text", "text": "hello", "annotations": [], "logprobs": []},
                    {"type": "output_text", "text": " world", "annotations": [], "logprobs": []}
                ]
            }
        ],
        "usage": {
            "input_tokens": 8,
            "output_tokens": 2,
            "total_tokens": 10,
            "input_tokens_details": {"cached_tokens": 3},
            "output_tokens_details": {"reasoning_tokens": 1}
        }
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let translated: CreateChatCompletionResponse = (&response).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["id"], "chatcmpl_resp_42");
    assert_eq!(value["model"], "glm-5.1");
    assert_eq!(value["object"], "chat.completion");
    assert_eq!(value["created"], 1700000000);
    assert_eq!(value["choices"][0]["message"]["role"], "assistant");
    assert_eq!(value["choices"][0]["message"]["content"], "hello world");
    assert_eq!(value["choices"][0]["finish_reason"], "stop");
    assert_eq!(value["usage"]["prompt_tokens"], 8);
    assert_eq!(value["usage"]["completion_tokens"], 2);
    assert_eq!(value["usage"]["total_tokens"], 10);
    assert_eq!(value["usage"]["prompt_tokens_details"]["cached_tokens"], 3);
    assert_eq!(
        value["usage"]["completion_tokens_details"]["reasoning_tokens"],
        1
    );
}

#[test]
fn translates_responses_function_call_to_chat_tool_calls() {
    let upstream = json!({
        "id": "resp_1",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {"type": "function_call", "id": "fc_1", "call_id": "call_abc", "name": "lookup", "arguments": "{\"id\":\"42\"}"}
        ]
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let translated: CreateChatCompletionResponse = (&response).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(
        value["choices"][0]["message"]["content"],
        serde_json::Value::Null
    );
    // The wire-level `call_id` is the pairing identifier used across Chat
    // tool_calls/tool messages; prefer it over the optional item `id`.
    assert_eq!(
        value["choices"][0]["message"]["tool_calls"][0]["id"],
        "call_abc"
    );
    assert_eq!(value["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        value["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
        "lookup"
    );
}

#[test]
fn translates_responses_incomplete_status_to_length_finish_reason() {
    let upstream = json!({
        "id": "resp_1",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "incomplete",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {"type": "message", "id": "m", "role": "assistant", "status": "incomplete", "content": [{"type": "output_text", "text": "partial", "annotations": [], "logprobs": []}]}
        ]
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let translated: CreateChatCompletionResponse = (&response).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert_eq!(value["choices"][0]["finish_reason"], "length");
}

#[test]
fn rejects_unknown_responses_incomplete_reason_at_protocol_boundary() {
    let upstream = json!({
        "id": "resp_1",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "incomplete",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": {"reason": "provider_shutdown"},
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {"type": "message", "id": "m", "role": "assistant", "status": "incomplete", "content": [{"type": "output_text", "text": "partial", "annotations": [], "logprobs": []}]}
        ]
    });
    let error = serde_json::from_value::<Response>(upstream).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("unknown variant `provider_shutdown`")
    );
}

#[test]
fn translates_responses_refusal_to_chat_refusal_message() {
    let upstream = json!({
        "id": "resp_refusal",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [{
            "type": "message",
            "id": "m",
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "refusal", "refusal": "I can't help with that."}]
        }]
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let translated: CreateChatCompletionResponse = (&response).try_into().unwrap();
    let value = serde_json::to_value(translated).unwrap();

    assert!(value["choices"][0]["message"]["content"].is_null());
    assert_eq!(
        value["choices"][0]["message"]["refusal"],
        "I can't help with that."
    );
    assert_eq!(value["choices"][0]["finish_reason"], "stop");
}

#[test]
fn rejects_mixed_responses_text_and_refusal_for_chat_response() {
    let upstream = json!({
        "id": "resp_mixed_refusal",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [{
            "type": "message",
            "id": "m",
            "role": "assistant",
            "status": "completed",
            "content": [
                {"type": "output_text", "text": "partial", "annotations": [], "logprobs": []},
                {"type": "refusal", "refusal": "I can't help with that."}
            ]
        }]
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let error: Result<CreateChatCompletionResponse, _> = (&response).try_into();
    let error = error.unwrap_err().to_string();

    assert!(error.contains("both text and refusal content"));
}

#[test]
fn translates_responses_reasoning_output_to_chat_reasoning_content() {
    let upstream = json!({
        "id": "resp_reasoning",
        "model": "gpt-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [{
            "type": "reasoning",
            "id": "r",
            "summary": [{"type": "summary_text", "text": "Summary. "}],
            "content": [{"type": "reasoning_text", "text": "Details."}],
            "status": "completed"
        }]
    });
    let value = super::super::translate_non_streaming_response(upstream).unwrap();

    assert_eq!(
        value["choices"][0]["message"]["reasoning_content"],
        "Summary. Details."
    );
    assert!(value["choices"][0]["message"]["content"].is_null());
}

#[test]
fn rejects_responses_output_without_chat_content() {
    let upstream = json!({
        "id": "resp_1",
        "model": "glm-5.1",
        "created_at": 0,
        "status": "completed",
        "object": "response",
        "metadata": null,
        "temperature": null,
        "top_p": null,
        "error": null,
        "incomplete_details": null,
        "instructions": null,
        "parallel_tool_calls": false,
        "tool_choice": "auto",
        "tools": [],
        "output": [
            {"type": "reasoning", "id": "r", "summary": [], "content": []}
        ]
    });
    let response = serde_json::from_value::<Response>(upstream).unwrap();
    let error: Result<CreateChatCompletionResponse, _> = (&response).try_into();
    let error = error.unwrap_err().to_string();
    assert!(error.contains("no Chat-representable text, reasoning, or tool calls"));
}
