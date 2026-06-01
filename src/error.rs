use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::response::{IntoResponse, Json};
use std::path::PathBuf;
use thiserror::Error as ThisError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, ThisError)]
pub enum RequestError {
    /// Reading the inbound HTTP request body failed.
    #[error("request body error: {0}")]
    Body(#[from] axum::Error),

    /// The client request itself is invalid for the ingress protocol.
    #[error("invalid request: {0}")]
    Invalid(String),
}

#[derive(Debug, ThisError)]
pub enum ConfigError {
    /// Reading the config file from disk failed.
    #[error("read config file `{path}`", path = .path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The config file was readable, but its contents are invalid.
    #[error(
        "invalid config file `{path}`\n\n{message}\n\nFix this file, compare it with `config.example.toml` in the same directory, or delete it to regenerate defaults.",
        path = .path.display()
    )]
    Invalid { path: PathBuf, message: String },
}

#[derive(Debug, ThisError)]
pub enum InternalError {
    /// Internal URL construction failed while preparing an upstream request.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Serializing a normalized or rewritten JSON payload failed.
    #[error("serialize request body: {0}")]
    Json(#[from] serde_json::Error),

    /// Local filesystem access failed outside of config-file reads.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Building an upstream HTTP client during runtime assembly failed.
    #[error("build upstream HTTP client: {0}")]
    HttpClientBuild(#[source] reqwest::Error),

    /// Default provider names or resolved provider selections were invalid.
    #[error("invalid provider resolution: {0}")]
    InvalidProviderResolution(String),

    /// A route pattern or route-level transformation rule is invalid.
    #[error("invalid route configuration: {0}")]
    InvalidRoute(String),
}

#[derive(Debug, ThisError)]
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

    /// Sending an already-validated request to the upstream failed.
    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),
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

impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        match self {
            Error::Request(error) => {
                let message = match error {
                    RequestError::Body(error) => error.to_string(),
                    RequestError::Invalid(message) => message,
                };
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": {
                            "message": message,
                            "type": "invalid_request_error"
                        }
                    })),
                )
                    .into_response()
            }
            Error::Upstream(error) => (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": {
                        "message": error.to_string(),
                        "type": "upstream_error"
                    }
                })),
            )
                .into_response(),
            Error::Config(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": error.to_string(),
                        "type": "internal_error"
                    }
                })),
            )
                .into_response(),
            Error::Internal(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": error.to_string(),
                        "type": "internal_error"
                    }
                })),
            )
                .into_response(),
        }
    }
}
