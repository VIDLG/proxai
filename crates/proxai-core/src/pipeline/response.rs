use std::sync::Arc;

use futures_util::{Stream, StreamExt};
use serde_json::Value;

use crate::observe::Observer;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::provider::{
    ProviderBehavior, normalize_provider_response, normalize_provider_stream_event,
    requires_structured_normalization,
};
use crate::translation::stream::{
    StreamEventStream, StreamTranslationInput, StreamTranslationResult,
};
use crate::translation::{TranslationResult, Translator};

/// Carrier-independent provider response normalization and translation for one
/// request/provider protocol pair.
#[derive(Clone)]
pub struct ResponsePipeline {
    behavior: ProviderBehavior,
    translator: Translator,
}

impl ResponsePipeline {
    pub fn new(request_protocol: RequestProtocol, behavior: ProviderBehavior) -> Self {
        Self {
            behavior,
            translator: Translator::new(request_protocol, behavior.protocol()),
        }
    }

    pub fn with_observer(mut self, observer: Arc<dyn Observer>) -> Self {
        self.translator = self.translator.with_observer(observer);
        self
    }

    pub fn request_protocol(&self) -> RequestProtocol {
        self.translator.route().request_protocol
    }

    pub fn provider_protocol(&self) -> ProviderProtocol {
        self.behavior.protocol()
    }

    /// Returns whether a carrier must decode the provider response into
    /// structured values/events before forwarding it to the client.
    pub fn requires_structured_processing(&self) -> bool {
        !self
            .translator
            .route()
            .request_protocol
            .matches_provider_protocol(self.behavior.protocol())
            || requires_structured_normalization(self.behavior)
    }

    pub fn translate_response(&self, payload: Value) -> TranslationResult<Value> {
        self.translator
            .translate_response(normalize_provider_response(
                self.behavior,
                payload,
                self.translator.observer().as_ref(),
            ))
    }

    pub fn translate_stream<S>(self, input: S) -> StreamEventStream
    where
        S: Stream<Item = StreamTranslationResult<StreamTranslationInput>> + Send + 'static,
    {
        let Self {
            behavior,
            translator,
        } = self;
        let observer = Arc::clone(translator.observer());
        let normalized = input.map(move |item| {
            item.map(|input| match input {
                StreamTranslationInput::Event(event) => StreamTranslationInput::Event(
                    normalize_provider_stream_event(behavior, event, observer.as_ref()),
                ),
                StreamTranslationInput::End(end) => StreamTranslationInput::End(end),
            })
        });
        translator.translate_stream(normalized)
    }
}
