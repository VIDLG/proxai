use async_stream::try_stream;
use bytes::{Bytes, BytesMut};
use futures_util::{Stream, StreamExt};
use getset::Getters;
use serde::Serialize;
use serde_json::Value;
use sse_core::{SseDecoder, SseEvent as DecodedSseEvent};

use crate::http_support::{ByteStreamError, boxed_stream_error};

pub(crate) type SseResult<T> = Result<T, SseError>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SseError {
    #[error("SSE decode failed: {0}")]
    Decode(String),

    #[error("SSE JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("SSE payload missing field: {0}")]
    MissingField(&'static str),
}

/// A complete raw SSE frame, including its terminating blank line.
///
/// Use this for wire-level transforms that must preserve non-target frames as
/// bytes. Incomplete EOF bytes are represented as `SseSegment::Tail`, not as a
/// frame.
#[derive(Debug, Clone, PartialEq, Eq, Getters)]
pub(crate) struct SseFrame {
    #[getset(get = "pub(crate)")]
    bytes: Bytes,
}

/// Output from raw SSE frame splitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SseSegment {
    Frame(SseFrame),
    Tail(Bytes),
}

/// A decoded SSE message event.
///
/// This intentionally models semantic event data rather than original wire
/// bytes. Formatting, comments, retry commands, and unknown SSE fields are not
/// preserved here; use `SseFrame` for wire-preserving transforms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SseEvent {
    /// Effective SSE event type.
    ///
    /// `sse-core` returns the SSE default `message` both when `event:` is
    /// omitted and when `event: message` is explicit. Treat `message` as a
    /// transport default: do not inject it into JSON payloads or re-emit it
    /// when preserving raw events.
    pub(crate) event_type: String,
    pub(crate) data: String,
}

#[derive(Debug, Default)]
pub(crate) struct SseEventScanner {
    buffer: BytesMut,
}

impl SseFrame {
    pub(crate) fn new(bytes: Bytes) -> Self {
        Self { bytes }
    }
}

impl TryFrom<&SseFrame> for Option<SseEvent> {
    type Error = SseError;

    fn try_from(frame: &SseFrame) -> SseResult<Self> {
        let mut decoder = SseDecoder::new();
        let mut buffer = BytesMut::from(frame.bytes().as_ref());
        while let Some(decoded) = decoder.next(&mut buffer) {
            let DecodedSseEvent::Message(message) =
                decoded.map_err(|error| SseError::Decode(error.to_string()))?
            else {
                continue;
            };

            if message.data.is_empty() {
                continue;
            }

            return Ok(Some(SseEvent {
                event_type: message.event.into_owned(),
                data: message.data,
            }));
        }

        Ok(None)
    }
}

impl SseEvent {
    pub(crate) const DEFAULT_EVENT_TYPE: &'static str = "message";

    pub(crate) fn is_default_event_type(&self) -> bool {
        self.event_type == Self::DEFAULT_EVENT_TYPE
    }

    pub(crate) fn is_done_sentinel(&self) -> bool {
        self.is_default_event_type() && self.data == "[DONE]"
    }

    pub(crate) fn matches_type_or_data(&self, event_type: &str) -> bool {
        self.event_type == event_type
            || self.payload_json().is_some_and(|payload| {
                payload.get("type").and_then(Value::as_str) == Some(event_type)
            })
    }

    pub(crate) fn payload_json(&self) -> Option<Value> {
        serde_json::from_str(&self.data).ok()
    }

    pub(crate) fn payload_with_type(&self) -> SseResult<Value> {
        let mut payload = serde_json::from_str::<Value>(&self.data)?;
        if !self.is_default_event_type()
            && let Some(object) = payload.as_object_mut()
        {
            object
                .entry("type".to_string())
                .or_insert_with(|| Value::String(self.event_type.clone()));
        }
        Ok(payload)
    }
}

impl SseEventScanner {
    /// Scan one upstream byte chunk and emit completed SSE message events.
    ///
    /// This first splits raw bytes into complete SSE frames, then decodes each
    /// frame into event semantics. Incomplete tail bytes remain buffered.
    pub(crate) fn scan(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        let mut events = Vec::new();

        for frame in self.scan_frames(chunk) {
            let Ok(Some(event)) = Option::<SseEvent>::try_from(&frame) else {
                continue;
            };
            events.push(event);
        }

        events
    }

    fn scan_frames(&mut self, chunk: &[u8]) -> Vec<SseFrame> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        while let Some(frame_end) = complete_sse_frame_end(&self.buffer) {
            frames.push(SseFrame::new(self.buffer.split_to(frame_end).freeze()));
        }

        frames
    }
}

/// Convert a byte stream into raw SSE segments.
///
/// Completed frames are yielded with their original bytes. Any incomplete tail
/// is yielded once at EOF so callers can preserve truncated upstream output.
/// This is useful for selective wire-level transforms where decoded event
/// semantics would lose formatting or unknown SSE fields.
pub(crate) fn sse_frame_stream<S, E>(
    input: S,
) -> impl Stream<Item = Result<SseSegment, ByteStreamError>> + Send
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Into<ByteStreamError> + Send + 'static,
{
    try_stream! {
        let mut input = Box::pin(input);
        let mut buffer = BytesMut::new();

        while let Some(chunk) = input.next().await {
            let chunk = chunk.map_err(Into::into)?;
            buffer.extend_from_slice(&chunk);
            while let Some(frame_end) = complete_sse_frame_end(&buffer) {
                yield SseSegment::Frame(SseFrame::new(buffer.split_to(frame_end).freeze()));
            }
        }

        if !buffer.is_empty() {
            yield SseSegment::Tail(buffer.freeze());
        }
    }
}

/// Convert a byte stream into decoded SSE message events.
///
/// This is the event-semantic layer above `sse_frame_stream`: complete frames
/// are decoded into events, while incomplete EOF tails are ignored.
pub(crate) fn sse_event_stream<S, E>(
    input: S,
) -> impl Stream<Item = Result<SseEvent, ByteStreamError>> + Send
where
    S: Stream<Item = Result<Bytes, E>> + Send + 'static,
    E: Into<ByteStreamError> + Send + 'static,
{
    try_stream! {
        let mut segments = Box::pin(sse_frame_stream(input));

        while let Some(segment) = segments.next().await {
            match segment? {
                SseSegment::Frame(frame) => {
                    if let Some(event) = Option::<SseEvent>::try_from(&frame).map_err(boxed_stream_error)? {
                        yield event;
                    }
                }
                SseSegment::Tail(_) => {}
            }
        }
    }
}

pub(crate) fn encode_sse_json<T>(event_type: &str, payload: &T) -> serde_json::Result<Bytes>
where
    T: Serialize,
{
    let data = serde_json::to_string(payload)?;
    Ok(Bytes::from(format!(
        "event: {event_type}\ndata: {data}\n\n"
    )))
}

fn complete_sse_frame_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(2)
        .position(|window| window == b"\n\n")
        .map(|position| position + 2)
        .or_else(|| {
            buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|position| position + 4)
        })
}

#[cfg(test)]
#[path = "sse_tests.rs"]
mod tests;
