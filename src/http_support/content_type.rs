use axum::http::{HeaderMap, HeaderValue, header};
use derive_more::Display;
use serde::Serialize;

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
    type Error = axum::http::header::ToStrError;

    fn try_from(value: &HeaderValue) -> std::result::Result<Self, Self::Error> {
        let value = value.to_str()?;
        Ok(if value.starts_with("text/event-stream") {
            Self::EventStream
        } else {
            Self::Other(value.to_string())
        })
    }
}
