use serde_json::json;

use super::{
    ChatRequestExtensions, inject_response_reasoning, response_reasoning, stream_reasoning,
};

#[test]
fn extracts_and_reapplies_zed_assistant_reasoning_content() {
    let source = json!({
        "model": "gpt-test",
        "messages": [
            {"role": "user", "content": "question"},
            {
                "role": "assistant",
                "content": "answer",
                "reasoning_content": "thinking"
            }
        ]
    });
    let extensions = ChatRequestExtensions::extract(&source).unwrap();
    assert_eq!(extensions.reasoning(1), Some("thinking"));

    let mut official = json!({
        "model": "gpt-test",
        "messages": [
            {"role": "user", "content": "question"},
            {"role": "assistant", "content": "answer"}
        ]
    });
    extensions.apply(&mut official).unwrap();
    assert_eq!(official["messages"][1]["reasoning_content"], "thinking");
}

#[test]
fn rejects_reasoning_content_on_non_assistant_messages() {
    let error = ChatRequestExtensions::extract(&json!({
        "messages": [{
            "role": "user",
            "content": "question",
            "reasoning_content": "invalid"
        }]
    }))
    .unwrap_err();

    assert!(error.to_string().contains("outside an assistant message"));
}

#[test]
fn rejects_non_string_reasoning_content() {
    let error = ChatRequestExtensions::extract(&json!({
        "messages": [{
            "role": "assistant",
            "content": "answer",
            "reasoning_content": {"unexpected": true}
        }]
    }))
    .unwrap_err();

    assert!(error.to_string().contains("must be a string or null"));
}

#[test]
fn reads_non_streaming_zed_reasoning_content() {
    let reasoning = response_reasoning(&json!({
        "choices": [{
            "message": {
                "content": null,
                "refusal": null,
                "role": "assistant",
                "reasoning_content": "thinking"
            }
        }]
    }))
    .unwrap();

    assert_eq!(reasoning.as_deref(), Some("thinking"));
}

#[test]
fn ignores_stream_only_reasoning_alias_in_non_streaming_message() {
    let reasoning = response_reasoning(&json!({
        "choices": [{
            "message": {
                "content": null,
                "refusal": null,
                "role": "assistant",
                "reasoning": "stream-only alias"
            }
        }]
    }))
    .unwrap();

    assert_eq!(reasoning, None);
}

#[test]
fn reads_both_zed_supported_stream_reasoning_fields_in_zed_order() {
    let reasoning = stream_reasoning(&json!({
        "choices": [{
            "delta": {
                "reasoning": "first",
                "reasoning_content": " second"
            }
        }]
    }))
    .unwrap();

    assert_eq!(reasoning.as_deref(), Some("first second"));
}

#[test]
fn injects_non_streaming_reasoning_after_official_serialization() {
    let mut response = json!({
        "choices": [{"message": {"content": null, "refusal": null, "role": "assistant", "content": "answer"}}]
    });
    inject_response_reasoning(&mut response, Some("thinking".to_string())).unwrap();

    assert_eq!(
        response["choices"][0]["message"]["reasoning_content"],
        "thinking"
    );
}
