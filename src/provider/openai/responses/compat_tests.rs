use bytes::Bytes;
use serde::Deserialize;
use serde_json::Value;

use crate::sse::SseFrame;

use super::normalize_nested_error_sse_frame;

#[test]
fn normalizes_nested_generic_error_event_for_zed_responses_parser() {
    let frame = br#"event: error
data: {"type":"error","error":{"type":"invalid_request_error","code":"context_length_exceeded","message":"Your input exceeds the context window of this model.","param":"input"},"sequence_number":2}

"#;

    let normalized = normalize_nested_error_sse_frame(&sse_frame(frame)).unwrap();
    let normalized = std::str::from_utf8(&normalized).unwrap();

    assert!(normalized.contains("event: error"));
    assert!(normalized.contains(r#""type":"error""#));
    assert!(normalized.contains(r#""sequence_number":2"#));
    assert!(normalized.contains(r#""code":"context_length_exceeded""#));
    assert!(
        normalized.contains(r#""message":"Your input exceeds the context window of this model.""#)
    );
    assert!(normalized.contains(r#""param":"input""#));
    assert!(!normalized.contains(r#""error":"#));
}

#[test]
fn nested_generic_error_event_fails_zed_1_3_7_shape_before_compat() {
    let raw_data = r#"{"type":"error","error":{"type":"invalid_request_error","code":"context_length_exceeded","message":"Your input exceeds the context window of this model.","param":"input"},"sequence_number":2}"#;

    let error = serde_json::from_str::<Zed137ResponsesStreamEvent>(raw_data).unwrap_err();

    assert!(error.to_string().contains("missing field `message`"));
}

#[test]
fn normalized_nested_generic_error_event_matches_zed_1_3_7_shape() {
    let frame = br#"event: error
data: {"type":"error","error":{"type":"invalid_request_error","code":"context_length_exceeded","message":"Your input exceeds the context window of this model.","param":"input"},"sequence_number":2}

"#;

    let normalized = normalize_nested_error_sse_frame(&sse_frame(frame)).unwrap();
    let data = sse_data(&normalized);
    let parsed = serde_json::from_str::<Zed137ResponsesStreamEvent>(&data).unwrap();

    let Zed137ResponsesStreamEvent::GenericError { error } = parsed else {
        panic!("expected zed generic error event");
    };
    assert_eq!(error.code.as_deref(), Some("context_length_exceeded"));
    assert_eq!(
        error.message,
        "Your input exceeds the context window of this model."
    );
    assert_eq!(error.param.as_ref().and_then(Value::as_str), Some("input"));
}

#[test]
fn standard_response_error_event_is_not_rewritten() {
    let frame = br#"event: response.error
data: {"type":"response.error","error":{"code":"bad_request","message":"standard response error","param":"input"},"sequence_number":2}

"#;

    assert!(normalize_nested_error_sse_frame(&sse_frame(frame)).is_none());

    let parsed = serde_json::from_str::<Zed137ResponsesStreamEvent>(&sse_data(frame)).unwrap();
    let Zed137ResponsesStreamEvent::Error { error } = parsed else {
        panic!("expected zed response.error event");
    };
    assert_eq!(error.message, "standard response error");
}

#[test]
fn top_level_generic_error_event_is_not_rewritten() {
    let frame = br#"event: error
data: {"type":"error","code":"bad_request","message":"already compatible","param":"input","sequence_number":2}

"#;

    assert!(normalize_nested_error_sse_frame(&sse_frame(frame)).is_none());
}

fn sse_frame(bytes: &'static [u8]) -> SseFrame {
    SseFrame::new(Bytes::from_static(bytes))
}

fn sse_data(frame: &[u8]) -> String {
    let frame = std::str::from_utf8(frame).unwrap();
    frame
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum Zed137ResponsesStreamEvent {
    #[serde(rename = "response.error")]
    Error { error: Zed137ResponseError },
    #[serde(rename = "error")]
    GenericError {
        #[serde(flatten)]
        error: Zed137ResponseError,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug)]
struct Zed137ResponseError {
    #[serde(default)]
    code: Option<String>,
    message: String,
    #[serde(default)]
    param: Option<Value>,
}
