use crate::routing::RoutingError;
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

    /// Routing configuration or request resolution failed in the core router.
    #[error(transparent)]
    Routing(#[from] RoutingError),

    /// The immutable provider registry diverged from the validated core router.
    #[error("routed provider `{provider}` has no configured transport")]
    MissingProviderTransport { provider: String },

    /// Protocol translation failed for a configured route.
    #[error(transparent)]
    Translation(#[from] TranslationError),
}
