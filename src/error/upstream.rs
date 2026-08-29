use std::fmt;

use axum::body::Bytes;
use proxai_core::provider::{ProviderError, normalize_provider_error};
use serde_json::Value;

use crate::http_support::UpstreamResponseHead;
use crate::protocol::ProviderProtocol;

#[derive(Debug, Clone)]
pub enum UpstreamResponseError {
    Provider(ProviderError),
    EmptyBody,
    NonJsonBody { text: String },
    UnknownBodyShape { text: String },
}

impl fmt::Display for UpstreamResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => {
                write!(f, "upstream response error: {}", error.message)?;
                if let Some(code) = &error.code {
                    write!(f, " code={code}")?;
                }
                if let Some(param) = &error.param {
                    match param {
                        Value::String(value) => write!(f, " param={value}"),
                        value => write!(f, " param={value}"),
                    }?;
                }
                Ok(())
            }
            Self::EmptyBody => write!(
                f,
                "proxy could not parse upstream error response: empty body"
            ),
            Self::NonJsonBody { text } => {
                write!(
                    f,
                    "proxy could not parse upstream error response as json: {text}"
                )
            }
            Self::UnknownBodyShape { text } => {
                write!(
                    f,
                    "proxy could not normalize upstream error response shape: {text}"
                )
            }
        }
    }
}

impl std::error::Error for UpstreamResponseError {}

impl UpstreamResponseError {
    pub(crate) fn parse_body(protocol: ProviderProtocol, bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes).trim().to_string();
        if text.is_empty() {
            return Self::EmptyBody;
        }

        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Self::NonJsonBody { text };
        };

        normalize_provider_error(protocol, &value)
            .map(Self::Provider)
            .unwrap_or(Self::UnknownBodyShape { text })
    }

    pub(crate) fn upstream_code(&self) -> Option<&str> {
        match self {
            Self::Provider(error) => error.code.as_deref(),
            Self::EmptyBody | Self::NonJsonBody { .. } | Self::UnknownBodyShape { .. } => None,
        }
    }

    pub(crate) fn upstream_message(&self) -> Option<&str> {
        match self {
            Self::Provider(error) => Some(&error.message),
            Self::EmptyBody | Self::NonJsonBody { .. } | Self::UnknownBodyShape { .. } => None,
        }
    }

    pub(crate) fn upstream_param(&self) -> Option<&Value> {
        match self {
            Self::Provider(error) => error.param.as_ref(),
            Self::EmptyBody | Self::NonJsonBody { .. } | Self::UnknownBodyShape { .. } => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UpstreamError {
    /// Sending an already-validated request to the upstream failed before a
    /// response head was available.
    #[error("upstream request failed: {0}")]
    RequestSend(#[source] reqwest::Error),

    /// The upstream returned a non-success status and a response body that was
    /// captured and parsed into proxai's compact diagnostic shape.
    #[error("upstream returned {}: {parsed}", head.status)]
    ErrorStatus {
        head: Box<UpstreamResponseHead>,
        body: Bytes,
        parsed: Box<UpstreamResponseError>,
    },

    /// A response head was available, but proxai could not read the response
    /// body. This is kept separate from request-send failure so diagnostics can
    /// retain upstream status/header context.
    #[error("upstream response body read failed: {source}")]
    ResponseBodyRead {
        head: Box<UpstreamResponseHead>,
        #[source]
        source: reqwest::Error,
    },
}

#[cfg(test)]
#[path = "upstream_tests.rs"]
mod tests;
