use axum::body::Bytes;
use serde_json::Value;

use crate::http_support::UpstreamResponseHead;

#[derive(Debug, Clone, thiserror::Error)]
pub enum UpstreamResponseError {
    #[error("upstream response error: {message}")]
    Upstream {
        code: Option<String>,
        message: String,
        param: Option<Value>,
    },
    #[error("proxy could not parse upstream error response: empty body")]
    EmptyBody,
    #[error("proxy could not parse upstream error response as json: {text}")]
    NonJsonBody { text: String },
    #[error("proxy could not normalize upstream error response shape: {text}")]
    UnknownBodyShape { text: String },
}

impl UpstreamResponseError {
    pub(crate) fn parse_body(bytes: &[u8]) -> Self {
        let text = String::from_utf8_lossy(bytes).trim().to_string();
        if text.is_empty() {
            return Self::EmptyBody;
        }

        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Self::NonJsonBody { text };
        };

        let code = value
            .pointer("/error/code")
            .or_else(|| value.pointer("/code"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let param = value
            .pointer("/error/param")
            .or_else(|| value.pointer("/param"))
            .cloned();

        if let Some(message) = value
            .pointer("/error/message")
            .or_else(|| value.pointer("/error"))
            .or_else(|| value.pointer("/detail"))
            .or_else(|| value.pointer("/message"))
            .and_then(Value::as_str)
        {
            return Self::Upstream {
                code,
                message: message.to_string(),
                param,
            };
        }
        if let Some(array) = value.pointer("/detail").and_then(Value::as_array) {
            let joined = array
                .iter()
                .filter_map(|item| {
                    item.get("msg")
                        .or_else(|| item.get("message"))
                        .and_then(Value::as_str)
                })
                .collect::<Vec<_>>()
                .join("; ");
            if !joined.is_empty() {
                return Self::Upstream {
                    code,
                    message: joined,
                    param,
                };
            }
        }

        Self::UnknownBodyShape { text }
    }

    pub(crate) fn upstream_code(&self) -> Option<&str> {
        match self {
            Self::Upstream { code, .. } => code.as_deref(),
            Self::EmptyBody | Self::NonJsonBody { .. } | Self::UnknownBodyShape { .. } => None,
        }
    }

    pub(crate) fn upstream_message(&self) -> Option<&str> {
        match self {
            Self::Upstream { message, .. } => Some(message),
            Self::EmptyBody | Self::NonJsonBody { .. } | Self::UnknownBodyShape { .. } => None,
        }
    }

    pub(crate) fn upstream_param(&self) -> Option<&Value> {
        match self {
            Self::Upstream { param, .. } => param.as_ref(),
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
        head: UpstreamResponseHead,
        body: Bytes,
        parsed: UpstreamResponseError,
    },

    /// A response head was available, but proxai could not read the response
    /// body. This is kept separate from request-send failure so diagnostics can
    /// retain upstream status/header context.
    #[error("upstream response body read failed: {source}")]
    ResponseBodyRead {
        head: UpstreamResponseHead,
        #[source]
        source: reqwest::Error,
    },
}
