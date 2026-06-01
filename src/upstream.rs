use axum::http::{HeaderMap, HeaderValue, StatusCode};
use derive_more::Display;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub(crate) struct UpstreamResponseHead {
    pub(crate) status: StatusCode,
    /// Parsed `Content-Type` header when present and readable.
    ///
    /// This can be absent for upstream errors, empty responses, or providers
    /// that omit response metadata. If present, proxai only classifies SSE
    /// specially; all other media types are preserved as raw strings.
    pub(crate) content_type: Option<ContentType>,
    /// Parsed `Content-Length` header when the upstream declares a fixed body size.
    ///
    /// This is commonly absent for SSE, chunked transfer, compressed responses,
    /// and HTTP/2+ responses where the transport framing carries body length.
    /// `None` means unknown length, not zero bytes.
    pub(crate) content_length: Option<u64>,
    /// Raw `Transfer-Encoding` header when present and readable.
    ///
    /// Most fixed-length responses omit this; HTTP/2+ responses usually omit
    /// `Transfer-Encoding: chunked` because framing is handled by the protocol.
    /// Kept as a raw string because proxai only logs/captures it for diagnostics.
    pub(crate) transfer_encoding: Option<String>,
    /// TTFB (`Time To First Byte`): approximate time until the upstream starts responding.
    ///
    /// `reqwest::Response` is available once response headers arrive, before the
    /// full body is read, so this is effectively time-to-response-headers in
    /// proxai. Compare it with snapshot `duration_ms` to tell whether latency is
    /// before the first upstream response or during body streaming/download.
    pub(crate) ttfb: Duration,
}

impl UpstreamResponseHead {
    pub(crate) fn from_headers(status: StatusCode, headers: &HeaderMap, ttfb: Duration) -> Self {
        Self {
            status,
            content_type: headers
                .get(http::header::CONTENT_TYPE)
                .and_then(|value| ContentType::try_from(value).ok()),
            content_length: headers
                .get(http::header::CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok()),
            transfer_encoding: headers
                .get(http::header::TRANSFER_ENCODING)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
            ttfb,
        }
    }

    pub(crate) fn with_content_length(mut self, content_length: u64) -> Self {
        self.content_length = Some(content_length);
        self
    }

    pub(crate) fn is_sse(&self) -> bool {
        self.content_type.as_ref().is_some_and(ContentType::is_sse)
    }
}

#[derive(Debug, Clone, Display)]
pub(crate) enum ContentType {
    #[display("text/event-stream")]
    EventStream,
    #[display("{_0}")]
    Other(String),
}

impl ContentType {
    pub(crate) fn is_sse(&self) -> bool {
        matches!(self, Self::EventStream)
    }
}

impl TryFrom<&HeaderValue> for ContentType {
    type Error = axum::http::header::ToStrError;

    fn try_from(value: &HeaderValue) -> Result<Self, Self::Error> {
        let value = value.to_str()?;
        Ok(if value.starts_with("text/event-stream") {
            Self::EventStream
        } else {
            Self::Other(value.to_string())
        })
    }
}
