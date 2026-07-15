use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{ErrorResponseFields, upstream_response_error_fields};
use crate::error::UpstreamResponseError;

#[test]
fn generic_sse_error_matches_zed_responses_nested_error_shape() {
    let frame = encode_sse_event(ErrorResponseFields::stream_translation(
        "translation failed",
    ));
    let data = sse_data(&frame);

    let event = serde_json::from_str::<ZedResponsesStreamEvent>(&data).unwrap();
    let ZedResponsesStreamEvent::GenericError { error } = event else {
        panic!("expected generic Responses error event");
    };

    let error = error.into_response_error();
    assert_eq!(error.message, "translation failed");
    assert_eq!(error.code, None);
    assert_eq!(error.param, None);
}

#[test]
fn upstream_error_payload_preserves_code_and_param() {
    let fields = upstream_response_error_fields(
        StatusCode::TOO_MANY_REQUESTS,
        &UpstreamResponseError::Upstream {
            code: Some("rate_limit_exceeded".to_string()),
            message: "quota exhausted".to_string(),
            param: Some(json!("input")),
        },
    );
    let frame = encode_sse_event(fields);
    let data = sse_data(&frame);

    let event = serde_json::from_str::<ZedResponsesStreamEvent>(&data).unwrap();
    let ZedResponsesStreamEvent::GenericError { error } = event else {
        panic!("expected generic Responses error event");
    };

    let error = error.into_response_error();
    assert_eq!(error.message, "quota exhausted");
    assert_eq!(error.code.as_deref(), Some("rate_limit_exceeded"));
    assert_eq!(error.param, Some(json!("input")));
}

#[test]
fn text_upstream_error_payload_preserves_code_and_param_as_json() {
    let fields = upstream_response_error_fields(
        StatusCode::BAD_GATEWAY,
        &UpstreamResponseError::Upstream {
            code: Some("array_above_max_length".to_string()),
            message: "Invalid 'input[3].content': array too long.".to_string(),
            param: Some(json!("input[3].content")),
        },
    );

    let body = fields.payload.text_body();
    let parsed = serde_json::from_str::<serde_json::Value>(&body).unwrap();

    assert_eq!(
        parsed["error"]["message"],
        "Invalid 'input[3].content': array too long."
    );
    assert_eq!(parsed["error"]["type"], "upstream_error");
    assert_eq!(parsed["error"]["status"], 502);
    assert_eq!(parsed["error"]["code"], "array_above_max_length");
    assert_eq!(parsed["error"]["param"], "input[3].content");
}

#[test]
fn generic_sse_error_matches_zed_chat_completions_error_shape() {
    let frame = encode_sse_event(ErrorResponseFields::stream_translation(
        "translation failed",
    ));
    let data = sse_data(&frame);

    let result = serde_json::from_str::<ZedChatCompletionStreamResult>(&data).unwrap();
    let ZedChatCompletionStreamResult::Err { error } = result else {
        panic!("expected chat completions stream error");
    };

    assert_eq!(error.message, "translation failed");
}

fn encode_sse_event(fields: ErrorResponseFields) -> bytes::Bytes {
    fields.payload.encode_sse_event().unwrap()
}

fn sse_data(frame: &[u8]) -> String {
    std::str::from_utf8(frame)
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ZedResponsesStreamEvent {
    #[serde(rename = "error")]
    GenericError {
        #[serde(flatten)]
        error: ZedGenericStreamErrorPayload,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Default)]
struct ZedGenericStreamErrorPayload {
    #[serde(flatten)]
    top_level: PartialZedResponseError,
    #[serde(default)]
    error: Option<PartialZedResponseError>,
}

#[derive(Deserialize, Debug, Default)]
struct PartialZedResponseError {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    param: Option<Value>,
}

impl ZedGenericStreamErrorPayload {
    fn into_response_error(self) -> ZedResponseError {
        let nested = self.error.unwrap_or_default();
        ZedResponseError {
            code: self.top_level.code.or(nested.code),
            message: self
                .top_level
                .message
                .or(nested.message)
                .unwrap_or_default(),
            param: self.top_level.param.or(nested.param),
        }
    }
}

#[derive(Debug)]
struct ZedResponseError {
    code: Option<String>,
    message: String,
    param: Option<Value>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum ZedChatCompletionStreamResult {
    Ok {
        #[serde(rename = "choices")]
        _choices: Vec<Value>,
        #[serde(rename = "usage")]
        _usage: Option<Value>,
    },
    Err {
        error: ZedChatCompletionStreamError,
    },
}

#[derive(Deserialize, Debug)]
struct ZedChatCompletionStreamError {
    message: String,
}
