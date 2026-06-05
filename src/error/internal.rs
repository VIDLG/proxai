use crate::translation::TranslationError;

#[derive(Debug, thiserror::Error)]
pub enum InternalError {
    /// Internal URL construction failed while preparing an upstream request.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// Serializing JSON at an internal proxy boundary failed.
    #[error("JSON serialization failed: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    /// Reading an internal HTTP response body failed.
    #[error("HTTP body read failed: {0}")]
    HttpBodyRead(#[source] axum::Error),

    /// Local filesystem access failed outside of config-file reads.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Building an upstream HTTP client during runtime assembly failed.
    #[error("build upstream HTTP client: {0}")]
    HttpClientBuild(#[source] reqwest::Error),

    /// Default provider names or resolved provider selections were invalid.
    #[error("invalid provider resolution: {0}")]
    InvalidProviderResolution(String),

    /// Protocol translation failed for a configured route.
    #[error(transparent)]
    Translation(#[from] TranslationError),

    /// A route pattern or route-level transformation rule is invalid.
    #[error("invalid route configuration: {0}")]
    InvalidRoute(String),
}
