//! Carrier-independent orchestration across ingress, routing, translation, and
//! provider adaptation.
//!
//! HTTP path detection, byte parsing/framing, transport, authentication,
//! timeouts, and concrete observation sinks remain downstream responsibilities.

mod error;
mod request;
mod response;

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::observe::{NoopObserver, Observer};
use crate::provider::ProviderBehavior;
use crate::routing::{
    Result as RoutingResult, RoutingConfig, RoutingTable, normalize_provider_name,
};

pub use error::{PrepareRequestError, PrepareRequestResult, RequestTranslationError};
pub use request::{PreparedRequest, ResolvedProvider};
pub use response::ResponsePipeline;

/// Compiled carrier-independent pipeline.
///
/// Clones share the compiled routing table and provider behavior catalog, while
/// [`Self::with_observer`] binds an observer for one request/response exchange.
#[derive(Clone)]
pub struct Pipeline {
    routing: Arc<RoutingTable>,
    providers: Arc<BTreeMap<String, ProviderBehavior>>,
    observer: Arc<dyn Observer>,
}

impl Pipeline {
    /// Validate routing against the provider catalog and compile the pipeline.
    pub fn build(
        routing: RoutingConfig,
        providers: BTreeMap<String, ProviderBehavior>,
    ) -> RoutingResult<Self> {
        let routing = RoutingTable::build(routing, providers.keys())?;
        let providers = providers
            .into_iter()
            .map(|(name, behavior)| (normalize_provider_name(&name), behavior))
            .collect();
        Ok(Self {
            routing: Arc::new(routing),
            providers: Arc::new(providers),
            observer: Arc::new(NoopObserver),
        })
    }

    /// Bind a downstream observer to one request/response exchange.
    pub fn with_observer(mut self, observer: impl Observer + 'static) -> Self {
        self.observer = Arc::new(observer);
        self
    }
}

#[cfg(test)]
mod tests;
