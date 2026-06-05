use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use getset::{CopyGetters, Getters};
use headers::{ContentLength, HeaderMapExt};
use std::time::Duration;

use super::ContentType;
use super::header::filter_forwardable_response_headers;
use super::stream::ByteStream;

#[derive(Debug, Clone, CopyGetters, Getters)]
pub struct OutboundResponseHead {
    #[getset(get_copy = "pub")]
    pub(crate) status: StatusCode,
    #[getset(get = "pub")]
    pub(crate) headers: HeaderMap,
}

impl OutboundResponseHead {
    pub(crate) fn from_upstream(upstream: &UpstreamResponseHead) -> Self {
        Self {
            status: upstream.status,
            headers: filter_forwardable_response_headers(&upstream.headers),
        }
    }

    pub(crate) fn content_type(&self) -> Option<ContentType> {
        ContentType::from_headers(&self.headers)
    }

    pub(crate) fn into_parts(self) -> (StatusCode, HeaderMap) {
        (self.status, self.headers)
    }
}

#[derive(Debug, Clone, CopyGetters, Getters)]
pub struct UpstreamResponseHead {
    #[getset(get_copy = "pub")]
    pub(crate) status: StatusCode,
    #[getset(get = "pub")]
    pub(crate) headers: HeaderMap,
    /// TTFB (`Time To First Byte`): approximate time until the upstream starts responding.
    ///
    /// `reqwest::Response` is available once response headers arrive, before the
    /// full body is read, so this is effectively time-to-response-headers in
    /// proxai. Compare it with snapshot `duration_ms` to tell whether latency is
    /// before the first upstream response or during body streaming/download.
    #[getset(get_copy = "pub")]
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
            .get(axum::http::header::TRANSFER_ENCODING)
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

pub(crate) fn response_with_headers(
    status: StatusCode,
    headers: HeaderMap,
    body: Body,
) -> Response<Body> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

pub(crate) fn response_from_parts_with_body(
    mut parts: axum::http::response::Parts,
    body: Body,
    content_type: HeaderValue,
) -> Response<Body> {
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(header::CONTENT_TYPE, content_type);
    Response::from_parts(parts, body)
}

pub(crate) fn json_response_from_parts(
    parts: axum::http::response::Parts,
    body: Vec<u8>,
) -> Response<Body> {
    response_from_parts_with_body(
        parts,
        Body::from(body),
        HeaderValue::from_static("application/json"),
    )
}

pub(crate) fn sse_response_from_parts(
    parts: axum::http::response::Parts,
    stream: ByteStream,
) -> Response<Body> {
    response_from_parts_with_body(
        parts,
        Body::from_stream(stream),
        HeaderValue::from_static("text/event-stream"),
    )
}

pub(crate) fn headers_are_sse(headers: &HeaderMap) -> bool {
    ContentType::from_headers(headers).is_some_and(|content_type| content_type.is_sse())
}

pub(crate) fn response_is_sse(response: &reqwest::Response) -> bool {
    headers_are_sse(response.headers())
}
