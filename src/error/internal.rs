#[derive(Debug, thiserror::Error)]
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
