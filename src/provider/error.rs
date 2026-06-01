use crate::protocol::ErrorObject;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub(crate) enum UpstreamErrorBodyError {
    #[error("proxy could not parse upstream error response: empty body")]
    Empty,
    #[error("proxy could not parse upstream error response as json: {text}")]
    NonJson { text: String },
    #[error("proxy could not normalize upstream error response shape: {text}")]
    UnknownShape { text: String },
}

#[derive(Debug, Clone, Error)]
pub(crate) enum UpstreamResponseError {
    #[error("upstream response error")]
    Protocol(ErrorObject),
    #[error("{message}")]
    Proxy { message: String },
    #[error("{message}")]
    Stream { message: String },
    #[error("upstream stream ended with unfinished tool arguments")]
    UnfinishedTool { sequence_number: Option<u64> },
}

impl UpstreamResponseError {
    pub(crate) fn response_message(&self) -> String {
        match self {
            Self::Protocol(error) => error.message.clone(),
            _ => self.to_string(),
        }
    }

    pub(crate) fn response_type(&self) -> &'static str {
        match self {
            Self::Protocol(_) => "upstream_error",
            _ => "proxy_error",
        }
    }
}

impl From<UpstreamErrorBodyError> for UpstreamResponseError {
    fn from(error: UpstreamErrorBodyError) -> Self {
        Self::Proxy {
            message: error.to_string(),
        }
    }
}

/// Loosely normalizes upstream non-2xx error bodies into the compact error shape
/// proxai returns to clients and logs.
///
/// Upstream error bodies are not a stable protocol surface: they can come from the
/// provider, an OpenAI-compatible shim, a framework validator, a proxy/CDN, or a
/// load balancer. Common useful shapes include OpenAI-style
/// `{ "error": { "message": ... } }`, simple `{ "message": ... }`,
/// `{ "detail": ... }`, and FastAPI/Pydantic-style `{ "detail": [{ "msg": ... }] }`.
///
/// Keep this parser byte-oriented and schema-loose so captures can preserve the
/// original body and callers still get useful diagnostics when the body is empty,
/// non-JSON, or uses a provider-specific error shape.
pub(crate) fn normalize_upstream_error_body(
    bytes: &[u8],
) -> Result<ErrorObject, UpstreamErrorBodyError> {
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    if text.is_empty() {
        return Err(UpstreamErrorBodyError::Empty);
    }

    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Err(UpstreamErrorBodyError::NonJson { text });
    };

    let code = value
        .pointer("/error/code")
        .or_else(|| value.pointer("/code"))
        .and_then(Value::as_str)
        .map(str::to_string);

    if let Some(message) = value
        .pointer("/error/message")
        .or_else(|| value.pointer("/error"))
        .or_else(|| value.pointer("/detail"))
        .or_else(|| value.pointer("/message"))
        .and_then(Value::as_str)
    {
        return Ok(ErrorObject {
            code: code.unwrap_or_else(|| "upstream_error".to_string()),
            message: message.to_string(),
        });
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
            return Ok(ErrorObject {
                code: code.unwrap_or_else(|| "upstream_error".to_string()),
                message: joined,
            });
        }
    }

    Err(UpstreamErrorBodyError::UnknownShape { text })
}
