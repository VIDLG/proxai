use crate::error::JsonPayloadError;
use crate::protocol::RequestProtocol;

pub type Result<T> = std::result::Result<T, IngressError>;

/// Failures while normalizing or validating a structured inbound request.
#[derive(Debug, thiserror::Error)]
pub enum IngressError {
    /// The JSON value did not match the detected protocol's request wire type.
    #[error(transparent)]
    JsonPayload(#[from] JsonPayloadError),

    /// The protocol payload omitted its routing model or supplied only
    /// whitespace.
    #[error("{} requests must include a non-empty `model`.", protocol.human_name())]
    MissingModel { protocol: RequestProtocol },
}

impl IngressError {
    pub fn as_json_payload_error(&self) -> Option<&JsonPayloadError> {
        match self {
            Self::JsonPayload(error) => Some(error),
            Self::MissingModel { .. } => None,
        }
    }
}
