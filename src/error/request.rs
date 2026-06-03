#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    /// Reading the inbound HTTP request body failed.
    #[error("request body error: {0}")]
    Body(#[from] axum::Error),

    /// The client request itself is invalid for the ingress protocol.
    #[error("invalid request: {0}")]
    Invalid(String),
}
