use axum::body::Bytes;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::sse::{sse_frame_stream, SseEvent, SseFrame, SseSegment};

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
            SseSegment::Frame(frame) => {
                normalize_nested_error_sse_frame(&frame).unwrap_or_else(|| frame.into_bytes())
            }
            SseSegment::Tail(bytes) => bytes,
        })
    }))
}

fn normalize_nested_error_sse_frame(frame: &SseFrame) -> Option<Bytes> {
    let event = Option::<SseEvent>::try_from(frame).ok().flatten()?;
    let payload = serde_json::from_str::<NestedGenericErrorPayload>(&event.data).ok()?;
    if event.event_type != "error" && payload.kind.as_deref() != Some("error") {
        return None;
    }
    if payload.message.is_some() {
        return None;
    }

    // Some OpenAI-compatible upstreams emit request failures inside a 200 SSE stream
    // as `event: error` + `{"type":"error","error":{...}}`. Zed 1.3.7 parses
    // `type:error` as the generic Responses error shape, where `message` is a
    // top-level field, while `response.error` owns the nested `error` object shape.
    // Keep our protocol model strict and adapt this provider-local upstream gap on
    // the outbound wire so clients can deserialize the stream and see the real
    // upstream failure, such as `context_length_exceeded`.
    let error = payload.error?;
    let normalized = NormalizedGenericErrorPayload {
        kind: "error",
        sequence_number: payload.sequence_number,
        code: error.code,
        message: error.message,
        param: error.param,
    };
    crate::sse::encode_sse_json("error", &normalized).ok()
}

#[derive(Debug, Deserialize)]
struct NestedGenericErrorPayload {
    #[serde(rename = "type")]
    kind: Option<String>,
    sequence_number: Option<u64>,
    message: Option<String>,
    error: Option<NestedGenericError>,
}

#[derive(Debug, Deserialize)]
struct NestedGenericError {
    code: Option<String>,
    message: String,
    param: Option<String>,
}

#[derive(Debug, Serialize)]
struct NormalizedGenericErrorPayload {
    #[serde(rename = "type")]
    kind: &'static str,
    sequence_number: Option<u64>,
    code: Option<String>,
    message: String,
    param: Option<String>,
}

#[cfg(test)]
#[path = "compat_tests.rs"]
mod tests;
