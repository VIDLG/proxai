use axum::body::Body;
pub(crate) use axum::http::*;
use derive_more::Display;
use serde::Serialize;

pub(crate) fn filter_forwardable_headers(headers: &HeaderMap) -> HeaderMap {
    let mut forwardable_headers = HeaderMap::new();
    for (key, value) in headers {
        if !is_hop_by_hop_header(key.as_str()) {
            forwardable_headers.append(key, value.clone());
        }
    }
    forwardable_headers
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "accept-encoding"
            | "connection"
            | "content-length"
            | "host"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
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

pub(crate) fn is_forwardable_error_response_header(name: &HeaderName) -> bool {
    let name = name.as_str();
    name.eq_ignore_ascii_case("retry-after")
        || name.eq_ignore_ascii_case("x-request-id")
        || name.eq_ignore_ascii_case("request-id")
        || name.starts_with("x-ratelimit-")
        || name.starts_with("anthropic-ratelimit-")
        || name.eq_ignore_ascii_case("openai-processing-ms")
}

pub(crate) fn headers_are_sse(headers: &HeaderMap) -> bool {
    ContentType::from_headers(headers).is_some_and(|content_type| content_type.is_sse())
}

pub(crate) fn response_is_sse(response: &reqwest::Response) -> bool {
    headers_are_sse(response.headers())
}

#[derive(Debug, Clone, Display, Serialize)]
pub(crate) enum ContentType {
    #[display("text/event-stream")]
    EventStream,
    #[display("{_0}")]
    Other(String),
}

impl ContentType {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Option<Self> {
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| Self::try_from(value).ok())
    }

    pub(crate) fn is_sse(&self) -> bool {
        matches!(self, Self::EventStream)
    }
}

impl AsRef<str> for ContentType {
    fn as_ref(&self) -> &str {
        match self {
            Self::EventStream => "text/event-stream",
            Self::Other(value) => value,
        }
    }
}

impl TryFrom<&HeaderValue> for ContentType {
    type Error = header::ToStrError;

    fn try_from(value: &HeaderValue) -> std::result::Result<Self, Self::Error> {
        let value = value.to_str()?;
        Ok(if value.starts_with("text/event-stream") {
            Self::EventStream
        } else {
            Self::Other(value.to_string())
        })
    }
}
