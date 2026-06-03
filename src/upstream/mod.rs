use getset::CopyGetters;
use serde::Serialize;
use std::time::{Duration, Instant};
use thiserror::Error;

mod non_streaming;
mod streaming;

pub(crate) use non_streaming::forward_non_streaming_response;
pub(crate) use streaming::{BodyAction, BodyObserver, prepare_response_stream};

#[derive(Debug, Clone, Error)]
pub(crate) enum UpstreamStreamError {
    #[error("{message}")]
    Stream { message: String },
    #[error("upstream stream ended with unfinished tool arguments")]
    UnfinishedTool { sequence_number: Option<u64> },
}

#[derive(Debug, Clone, Copy, CopyGetters)]
pub(crate) struct UpstreamBodyStreamStats {
    started: Instant,
    #[getset(get_copy = "pub(crate)")]
    chunks: u64,
    #[getset(get_copy = "pub(crate)")]
    bytes: u64,
}

impl UpstreamBodyStreamStats {
    pub(crate) fn new(started: Instant) -> Self {
        Self {
            started,
            chunks: 0,
            bytes: 0,
        }
    }

    pub(crate) fn record_chunk(&mut self, chunk: &[u8]) {
        self.chunks += 1;
        self.bytes += chunk.len() as u64;
    }

    pub(crate) fn metrics(&self) -> UpstreamStreamMetrics {
        UpstreamStreamMetrics::new(self.started.elapsed(), self.chunks, self.bytes)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct UpstreamStreamMetrics {
    pub(crate) duration: Duration,
    pub(crate) chunks: u64,
    pub(crate) bytes: u64,
}

impl UpstreamStreamMetrics {
    pub(crate) fn new(duration: Duration, chunks: u64, bytes: u64) -> Self {
        Self {
            duration,
            chunks,
            bytes,
        }
    }

    pub(crate) fn duration_ms(self) -> u64 {
        self.duration.as_millis() as u64
    }

    pub(crate) fn avg_chunk_bytes(self) -> u64 {
        if self.bytes == 0 || self.chunks == 0 {
            0
        } else {
            self.bytes / self.chunks
        }
    }
}
