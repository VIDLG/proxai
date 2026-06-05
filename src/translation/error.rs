use crate::protocol::{ProviderProtocol, RequestProtocol};

pub(crate) type Result<T> = std::result::Result<T, TranslationError>;

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
}
