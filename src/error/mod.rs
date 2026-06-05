pub mod config;
pub mod internal;
mod render;
pub mod request;
pub mod upstream;

pub use config::ConfigError;
pub use internal::InternalError;
pub(crate) use render::ErrorResponseFields;
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
