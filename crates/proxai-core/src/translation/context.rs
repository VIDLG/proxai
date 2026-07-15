//! Phase-bound translation context available to dispatch and protocol-pair code.
//!
//! This module contains no façade or pair-state dependencies. `Translator`
//! derives a `TranslationScope` for each operation; pair functions receive only
//! a borrowed scope.

use std::sync::Arc;

use crate::observe::{
    Observation, Observer, TranslationObservation, TranslationObservationKind, TranslationPhase,
};
use crate::protocol::{ProviderProtocol, RequestProtocol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TranslationRoute {
    pub(crate) request_protocol: RequestProtocol,
    pub(crate) provider_protocol: ProviderProtocol,
}

#[derive(Clone)]
pub(crate) struct TranslationScope {
    route: TranslationRoute,
    phase: TranslationPhase,
    observer: Arc<dyn Observer>,
}

impl TranslationScope {
    pub(crate) fn new(
        route: TranslationRoute,
        phase: TranslationPhase,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Self {
            route,
            phase,
            observer,
        }
    }

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
