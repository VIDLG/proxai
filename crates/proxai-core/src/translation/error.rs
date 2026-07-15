use crate::protocol::{ProviderProtocol, RequestProtocol};

pub type Result<T> = std::result::Result<T, TranslationError>;

#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    #[error("{from} -> {to} request translation is not implemented yet")]
    UnsupportedRequestPair {
        from: RequestProtocol,
        to: ProviderProtocol,
    },

    #[error("{from} -> {to} response translation is not implemented yet")]
    UnsupportedResponsePair {
        from: ProviderProtocol,
        to: RequestProtocol,
    },

    #[error("invalid translation payload: {0}")]
    InvalidPayload(String),

    #[error("JSON conversion failed during translation: {0}")]
    Json(#[from] serde_json::Error),

    #[error(
        "failed to deserialize normalized translation payload for {context} at JSON path `{path}` (pretty line {line}, column {column}): {message}"
    )]
    JsonPayload {
        context: &'static str,
        path: String,
        message: String,
        line: usize,
        column: usize,
    },
}
