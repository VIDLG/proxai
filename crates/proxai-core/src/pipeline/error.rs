use crate::ingress::{IngressError, PreparedInboundRequest};
use crate::routing::RoutingError;
use crate::translation::TranslationError;

use super::request::ResolvedProvider;

pub type PrepareRequestResult<T> = Result<T, PrepareRequestError>;

/// Failure while running the carrier-independent request pipeline.
#[derive(Debug, thiserror::Error)]
pub enum PrepareRequestError {
    #[error(transparent)]
    Ingress(#[from] IngressError),

    #[error(transparent)]
    Routing(#[from] RoutingError),

    #[error(transparent)]
    Translation(Box<RequestTranslationError>),
}

/// Request translation failure with the prepared source payload and resolved
/// provider context retained for downstream diagnostics.
#[derive(Debug, thiserror::Error)]
#[error("{source}")]
pub struct RequestTranslationError {
    inbound: PreparedInboundRequest,
    provider: ResolvedProvider,
    #[source]
    source: TranslationError,
}

impl From<RequestTranslationError> for PrepareRequestError {
    fn from(error: RequestTranslationError) -> Self {
        Self::Translation(Box::new(error))
    }
}

impl RequestTranslationError {
    pub(super) fn new(
        inbound: PreparedInboundRequest,
        provider: ResolvedProvider,
        source: TranslationError,
    ) -> Self {
        Self {
            inbound,
            provider,
            source,
        }
    }

    pub fn inbound(&self) -> &PreparedInboundRequest {
        &self.inbound
    }

    pub fn provider(&self) -> &ResolvedProvider {
        &self.provider
    }

    pub fn translation_error(&self) -> &TranslationError {
        &self.source
    }

    pub fn into_translation_error(self) -> TranslationError {
        self.source
    }
}
