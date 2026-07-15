use delegate::delegate;
use getset::{CopyGetters, Getters};
use serde_json::Value;

use crate::ingress::{PreparedInboundRequest, prepare_inbound_request};

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{ProviderBehavior, ProviderCompatibility, prepare_provider_request};
use crate::routing::ResolvedRoute;
use crate::translation::Translator;

use super::error::{PrepareRequestResult, RequestTranslationError};
use super::{Pipeline, ResponsePipeline};

impl Pipeline {
    /// Prepare a request and bind this pipeline's observer to its response pipeline.
    pub fn prepare_request(
        &self,
        request_protocol: RequestProtocol,
        payload: Value,
    ) -> PrepareRequestResult<PreparedRequest> {
        let observer = self.observer.clone();
        let inbound = prepare_inbound_request(request_protocol, payload, observer.as_ref())?;
        let route = self.routing.resolve(inbound.protocol(), inbound.model())?;
        let behavior = *self
            .providers
            .get(&route.provider)
            .expect("validated routing table must resolve a configured provider");
        let provider = ResolvedProvider::new(route, behavior);

        let translator = Translator::new(inbound.protocol(), provider.protocol())
            .with_observer(observer.clone());
        let translated = match translator.translate_request(inbound.normalized_payload()) {
            Ok(payload) => payload,
            Err(error) => {
                return Err(RequestTranslationError::new(inbound, provider, error).into());
            }
        };
        let provider_payload = prepare_provider_request(
            provider.protocol(),
            translated,
            provider.upstream_model(),
            observer.as_ref(),
        );
        let response =
            ResponsePipeline::new(inbound.protocol(), provider.behavior()).with_observer(observer);

        Ok(PreparedRequest {
            inbound,
            provider,
            provider_payload,
            response,
        })
    }
}

/// Provider and model selected for one prepared request.
#[derive(Debug, Clone, PartialEq, Eq, Getters, CopyGetters)]
pub struct ResolvedProvider {
    #[getset(get = "pub")]
    name: String,
    #[getset(get = "pub")]
    route_name: Option<String>,
    #[getset(get = "pub")]
    upstream_model: String,
    #[getset(get_copy = "pub")]
    behavior: ProviderBehavior,
}

impl ResolvedProvider {
    fn new(route: ResolvedRoute, behavior: ProviderBehavior) -> Self {
        Self {
            name: route.provider,
            route_name: route.route_name,
            upstream_model: route.upstream_model,
            behavior,
        }
    }

    delegate! {
        to self.behavior {
            pub fn protocol(&self) -> ProviderProtocol;
            pub fn compatibility(&self) -> ProviderCompatibility;
        }
    }
}

/// Prepared carrier-independent request plus the matching response processor.
pub struct PreparedRequest {
    pub inbound: PreparedInboundRequest,
    pub provider: ResolvedProvider,
    pub provider_payload: Value,
    pub response: ResponsePipeline,
}
