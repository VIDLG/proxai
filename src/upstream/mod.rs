use delegate::delegate;
use getset::CopyGetters;
use serde::Serialize;
use std::time::{Duration, Instant};
use strum::AsRefStr;
use thiserror::Error;

mod non_streaming;
mod streaming;

pub(crate) use non_streaming::forward_non_streaming_response;
pub(crate) use streaming::{BodyAction, BodyObserver, prepare_response_stream};

#[derive(Debug, Clone, Error)]
pub(crate) enum UpstreamStreamError {
    #[error("{message}")]
    Stream {
        message: String,
        kind: UpstreamStreamErrorKind,
    },
    #[error("upstream stream ended with unfinished tool arguments")]
    UnfinishedTool { sequence_number: Option<u64> },
}

impl UpstreamStreamError {
    pub(crate) fn from_reqwest(error: &reqwest::Error) -> Self {
        Self::Stream {
            message: error.to_string(),
            kind: UpstreamStreamErrorKind::from_reqwest(error),
        }
    }
}

/// High-level category for `UpstreamStreamError::Stream`, classified from a
/// `reqwest::Error` so log readers can grep the failure mode without parsing
/// free-text error messages.
///
/// The enum has two producer groups:
/// - variants below `UpstreamOther` are populated by [`Self::from_reqwest`]
///   and ordered specific → broad (decode and body beat timeout and connect);
/// - `ToolArgumentStall` and `MalformedToolArgument` are set locally by the
///   OpenAI Responses observer for semantic tool-call failures that do not
///   come from reqwest at all. They must stay distinguishable from
///   `Timeout` (an HTTP/transport timeout) so operators reading logs know
///   whether to tune `read_idle_timeout_secs` or `tool_calls.timeout_secs`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, AsRefStr, Serialize)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum UpstreamStreamErrorKind {
    /// `reqwest::Error::is_decode()` — response body could not be decoded
    /// (the classic "error decoding response body" case).
    Decode,
    /// `reqwest::Error::is_body()` — body I/O layer error, not a decode.
    Body,
    /// `reqwest::Error::is_timeout()` — a timeout fired at the HTTP/transport
    /// layer (connect/read/request timeout). Distinct from
    /// [`Self::ToolArgumentStall`], which is an application-semantic timeout
    /// on tool argument streams.
    Timeout,
    /// `reqwest::Error::is_connect()` — connection-layer failure.
    Connection,
    /// A `reqwest::Error` that did not match any more specific `is_*` check
    /// above (request-build, redirect, status-code, etc.). Also the default
    /// category for locally-synthesized errors that don't carry a more specific
    /// classification yet; existing call sites should prefer the most specific
    /// variant (`ToolArgumentStall`, `MalformedToolArgument`).
    #[default]
    UpstreamOther,
    /// Upstream SSE stream stalled while streaming tool arguments. Synthesized
    /// locally by the OpenAI Responses observer when
    /// `[tool_calls].timeout_secs` fires; the upstream is still emitting
    /// bytes overall but tool argument deltas have stopped advancing.
    /// Indicates the operator should tune `tool_calls.timeout_secs`, not
    /// `read_idle_timeout_secs`.
    ToolArgumentStall,
    /// Upstream SSE stream emitted a semantically invalid tool-call argument
    /// chunk (e.g. malformed JSON, argument delta without a pending
    /// function_call). Synthesized locally by the OpenAI Responses observer;
    /// this is a protocol-shape error from the upstream, not a transport
    /// error.
    MalformedToolArgument,
}

impl UpstreamStreamErrorKind {
    /// Classify a `reqwest::Error` into a stable category. Order matters: more
    /// specific `is_decode` / `is_body` are checked before the broader
    /// `is_timeout` / `is_connect`. Locally-synthesized categories
    /// (`ToolArgumentStall`, `MalformedToolArgument`) are never returned here
    /// because they do not originate from reqwest.
    pub(crate) fn from_reqwest(error: &reqwest::Error) -> Self {
        if error.is_decode() {
            Self::Decode
        } else if error.is_body() {
            Self::Body
        } else if error.is_timeout() {
            Self::Timeout
        } else if error.is_connect() {
            Self::Connection
        } else {
            Self::UpstreamOther
        }
    }
}

#[derive(Debug, Clone, Copy, CopyGetters)]
pub(crate) struct UpstreamBodyStreamStats {
    started: Instant,
    #[getset(get_copy = "pub(crate)")]
    chunks: u64,
    #[getset(get_copy = "pub(crate)")]
    bytes: u64,
    /// Timestamp of the most recently received chunk, or `None` before the
    /// first chunk arrives. Used to compute inter-chunk gaps.
    last_chunk_at: Option<Instant>,
    /// Largest observed gap between two consecutive chunks (or between stream
    /// start and the first chunk). Helps diagnose stalls inside long streams
    /// where the total duration looks fine but one segment stalled.
    #[getset(get_copy = "pub(crate)")]
    max_chunk_gap: Duration,
}

impl UpstreamBodyStreamStats {
    delegate! {
        to self.metrics() {
            pub(crate) fn duration_ms(self) -> u128;
        }
    }

    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            chunks: 0,
            bytes: 0,
            last_chunk_at: None,
            max_chunk_gap: Duration::ZERO,
        }
    }

    pub(crate) fn record_chunk(&mut self, chunk: &[u8]) {
        // Track the largest inter-chunk gap. The first chunk is measured from
        // `started` (effectively TTFB on the body side), subsequent chunks
        // from the previous chunk's arrival.
        let now = Instant::now();
        let reference = self.last_chunk_at.unwrap_or(self.started);
        let gap = now.saturating_duration_since(reference);
        if gap > self.max_chunk_gap {
            self.max_chunk_gap = gap;
        }
        self.last_chunk_at = Some(now);

        self.chunks += 1;
        self.bytes += chunk.len() as u64;
    }

    pub(crate) fn metrics(&self) -> UpstreamStreamMetrics {
        UpstreamStreamMetrics::new(
            self.started.elapsed(),
            self.chunks,
            self.bytes,
            self.max_chunk_gap,
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct UpstreamStreamMetrics {
    pub(crate) duration: Duration,
    pub(crate) chunks: u64,
    pub(crate) bytes: u64,
    /// Largest observed gap between consecutive chunks. Useful for diagnosing
    /// stalls inside an otherwise-healthy long stream.
    pub(crate) max_chunk_gap: Duration,
}

impl UpstreamStreamMetrics {
    pub(crate) fn new(
        duration: Duration,
        chunks: u64,
        bytes: u64,
        max_chunk_gap: Duration,
    ) -> Self {
        Self {
            duration,
            chunks,
            bytes,
            max_chunk_gap,
        }
    }

    delegate! {
        to self.duration {
            #[call(as_millis)]
            pub(crate) fn duration_ms(self) -> u128;
        }
    }

    delegate! {
        to self.max_chunk_gap {
            #[call(as_millis)]
            pub(crate) fn max_chunk_gap_ms(self) -> u128;
        }
    }

    pub(crate) fn avg_chunk_bytes(self) -> u64 {
        if self.bytes == 0 || self.chunks == 0 {
            0
        } else {
            self.bytes / self.chunks
        }
    }
}
