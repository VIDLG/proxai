use axum::body::Bytes;
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

use crate::sse::{sse_frame_stream, SseEvent, SseSegment};

/// Rewrites a narrow upstream compatibility gap in OpenAI Responses SSE streams.
///
/// Some OpenAI-compatible upstreams report request failures inside an HTTP 200
/// SSE stream as `event: error` with `data: {"type":"error","error":{...}}`.
/// Zed's Responses stream parser expects generic `type:error` events to carry
/// `message`, `code`, and `param` at the top level, so that nested shape hides
/// the useful upstream error. This stream wrapper uses the shared raw SSE frame
/// stream to rewrite only that nested error shape and otherwise preserve frame
/// bytes.
///
/// This intentionally does not use `SseEventScanner` plus re-encoding: for non-error
/// frames we want wire-level pass-through, including event fields, whitespace,
/// and multi-line data layout.
pub(super) fn normalize_nested_error_sse_stream(
    input: impl Stream<Item = std::io::Result<Bytes>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = std::io::Result<Bytes>> + Send>> {
    Box::pin(sse_frame_stream(input).map(|segment| {
        segment.map(|segment| match segment {
            SseSegment::Frame(frame) => normalize_nested_error_sse_frame(frame.bytes())
                .unwrap_or_else(|| frame.into_bytes()),
            SseSegment::Tail(bytes) => bytes,
        })
    }))
}

pub(super) fn normalize_nested_error_sse_frame(frame: &[u8]) -> Option<Bytes> {
    let frame = std::str::from_utf8(frame).ok()?;
    let mut event_type = SseEvent::DEFAULT_EVENT_TYPE;
    let mut data_lines = Vec::new();

    for line in frame.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(value) = line.strip_prefix("event:") {
            event_type = value.trim_start();
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim_start());
        }
    }

    let data = data_lines.join("\n");
    let payload = serde_json::from_str::<Value>(&data).ok()?;
    if event_type != "error" && payload.get("type").and_then(Value::as_str) != Some("error") {
        return None;
    }
    if payload.get("message").and_then(Value::as_str).is_some() {
        return None;
    }

    // Some OpenAI-compatible upstreams emit request failures inside a 200 SSE stream
    // as `event: error` + `{"type":"error","error":{...}}`. Zed 1.3.7 parses
    // `type:error` as the generic Responses error shape, where `message` is a
    // top-level field, while `response.error` owns the nested `error` object shape.
    // Keep our protocol model strict and adapt this provider-local upstream gap on
    // the outbound wire so clients can deserialize the stream and see the real
    // upstream failure, such as `context_length_exceeded`.
    let error = payload.get("error")?.as_object()?;
    let message = error.get("message")?.as_str()?;
    let normalized = serde_json::json!({
        "type": "error",
        "sequence_number": payload.get("sequence_number").and_then(Value::as_u64),
        "code": error.get("code").and_then(Value::as_str),
        "message": message,
        "param": error.get("param").and_then(Value::as_str),
    });
    crate::sse::encode_sse_json("error", &normalized).ok()
}
