//! Shared carrier-independent error details used across core domains.

use getset::{CopyGetters, Getters};

/// Structured failure produced while deserializing a JSON value into a
/// protocol-specific wire type.
///
/// The JSON path and pretty-printed source location make schema drift
/// diagnosable without coupling core to logging, capture, or HTTP rendering.
#[derive(Debug, thiserror::Error, Getters, CopyGetters)]
#[error(
    "failed to deserialize {context} at JSON path `{path}` (pretty line {line}, column {column}): {source}"
)]
pub struct JsonPayloadError {
    #[getset(get_copy = "pub")]
    context: &'static str,
    #[getset(get = "pub")]
    path: String,
    #[getset(get_copy = "pub")]
    line: usize,
    #[getset(get_copy = "pub")]
    column: usize,
    #[source]
    source: serde_json::Error,
}

impl JsonPayloadError {
    pub fn new(context: &'static str, path: impl Into<String>, source: serde_json::Error) -> Self {
        let line = source.line();
        let column = source.column();
        Self {
            context,
            path: path.into(),
            line,
            column,
            source,
        }
    }
}
