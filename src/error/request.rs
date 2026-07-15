use proxai_core::ingress::IngressError;
use proxai_core::protocol::RequestProtocol;

#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    /// Reading the inbound HTTP request body failed.
    #[error("request body error: {0}")]
    Body(#[from] axum::Error),

    /// The HTTP request body was not valid JSON for the detected protocol.
    #[error("invalid `{protocol}` request body JSON: {source}")]
    InvalidJson {
        protocol: RequestProtocol,
        #[source]
        source: serde_json::Error,
    },

    /// Structured ingress normalization or validation rejected the payload.
    #[error(transparent)]
    Ingress(#[from] IngressError),

    /// The HTTP path did not identify a supported inbound protocol.
    #[error("unsupported request path `{path}`")]
    UnsupportedPath { path: String },
}
