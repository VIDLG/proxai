pub mod config;
pub mod internal;
mod render;
pub mod request;
pub mod upstream;

pub use config::ConfigError;
pub use internal::InternalError;
pub use request::RequestError;
pub use upstream::{UpstreamError, UpstreamResponseError};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A client-facing request validation error.
    #[error(transparent)]
    Request(#[from] RequestError),

    /// A config loading or config validation error.
    #[error(transparent)]
    Config(#[from] ConfigError),

    /// An internal runtime or invariant error inside the proxy.
    #[error(transparent)]
    Internal(#[from] InternalError),

    /// Communicating with the selected upstream failed or returned an upstream
    /// error response.
    #[error(transparent)]
    Upstream(#[from] Box<UpstreamError>),
}

impl From<UpstreamError> for Error {
    fn from(error: UpstreamError) -> Self {
        Self::Upstream(Box::new(error))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        InternalError::from(error).into()
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        InternalError::from(error).into()
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        InternalError::from(error).into()
    }
}

impl From<axum::Error> for Error {
    fn from(error: axum::Error) -> Self {
        RequestError::from(error).into()
    }
}
