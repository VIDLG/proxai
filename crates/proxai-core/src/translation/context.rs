//! Shared translation context available to dispatch and protocol-pair code.
//!
//! This module contains no façade or pair-state dependencies. `Translator` owns
//! one configured `TranslationContext`; each translation operation derives a
//! phase-bound `TranslationScope`. Pair functions receive only a borrowed scope.

use std::sync::Arc;

use crate::observe::{
    Observation, Observer, TranslationObservation, TranslationObservationKind, TranslationPhase,
};
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslationRoute {
    pub request_protocol: RequestProtocol,
    pub provider_protocol: ProviderProtocol,
}

#[derive(Clone)]
pub(crate) struct TranslationContext {
    route: TranslationRoute,
    observer: Arc<dyn Observer>,
}

impl TranslationContext {
    pub(crate) fn new(route: TranslationRoute, observer: Arc<dyn Observer>) -> Self {
        Self { route, observer }
    }

    pub(crate) fn with_observer(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observer = observer;
        self
    }

    pub(crate) fn route(&self) -> TranslationRoute {
        self.route
    }

    pub(crate) fn scope(&self, phase: TranslationPhase) -> TranslationScope {
        TranslationScope {
            route: self.route,
            phase,
            observer: Arc::clone(&self.observer),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TranslationScope {
    route: TranslationRoute,
    phase: TranslationPhase,
    observer: Arc<dyn Observer>,
}

impl TranslationScope {
    pub(crate) fn route(&self) -> TranslationRoute {
        self.route
    }

    fn emit(
        &self,
        kind: TranslationObservationKind,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let observation = TranslationObservation {
            request_protocol: self.route.request_protocol,
            provider_protocol: self.route.provider_protocol,
            phase: self.phase,
            kind,
            subject: subject.into(),
            detail: detail.into(),
        };
        self.observer.observe(&Observation::from(observation));
    }

    pub(crate) fn dropped(&self, subject: impl Into<String>, detail: impl Into<String>) {
        self.emit(TranslationObservationKind::Dropped, subject, detail);
    }

    pub(crate) fn adapted(&self, subject: impl Into<String>, detail: impl Into<String>) {
        self.emit(TranslationObservationKind::Adapted, subject, detail);
    }
}
