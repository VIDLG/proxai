use axum::http::{HeaderMap, StatusCode};
use headers::{ContentLength, HeaderMapExt};
use std::time::Duration;

use crate::http_utils::ContentType;

#[derive(Debug, Clone)]
pub struct UpstreamResponseHead {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    /// TTFB (`Time To First Byte`): approximate time until the upstream starts responding.
    ///
    /// `reqwest::Response` is available once response headers arrive, before the
    /// full body is read, so this is effectively time-to-response-headers in
    /// proxai. Compare it with snapshot `duration_ms` to tell whether latency is
    /// before the first upstream response or during body streaming/download.
    pub(crate) ttfb: Duration,
}

impl UpstreamResponseHead {
    pub(crate) fn from_response(response: &reqwest::Response, ttfb: Duration) -> Self {
        Self::from_headers(response.status(), response.headers(), ttfb)
    }

    pub fn from_headers(status: StatusCode, headers: &HeaderMap, ttfb: Duration) -> Self {
        Self {
            status,
            headers: headers.clone(),
            ttfb,
        }
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn ttfb(&self) -> Duration {
        self.ttfb
    }

    pub(crate) fn content_type(&self) -> Option<ContentType> {
        ContentType::from_headers(&self.headers)
    }

    pub fn content_length(&self) -> Option<u64> {
        self.headers
            .typed_get::<ContentLength>()
            .map(|value| value.0)
    }

    pub fn transfer_encoding(&self) -> Option<&str> {
        self.headers
            .get(http::header::TRANSFER_ENCODING)
            .and_then(|value| value.to_str().ok())
    }

    pub fn content_type_text(&self) -> String {
        self.content_type()
            .map(|value| value.to_string())
            .unwrap_or_default()
    }

    pub fn transfer_encoding_text(&self) -> String {
        self.transfer_encoding().unwrap_or_default().to_string()
    }

    pub fn is_sse(&self) -> bool {
        self.content_type()
            .is_some_and(|content_type| content_type.is_sse())
    }
}
