//! Shared translation context available to dispatch and protocol-pair code.
//!
//! This module contains no façade or pair-state dependencies. `Translator` owns
//! one configured `TranslationContext`; each translation operation derives a
//! phase-bound `TranslationScope`. Pair functions receive only a borrowed scope.

use std::sync::Arc;

use crate::protocol::{ProviderProtocol, RequestProtocol};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslationRoute {
    pub request_protocol: RequestProtocol,
    pub provider_protocol: ProviderProtocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationPhase {
    Request,
    NonStreamingResponse,
    StreamingResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationObservationKind {
    Dropped,
    Adapted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationObservation {
    pub route: TranslationRoute,
    pub phase: TranslationPhase,
    pub kind: TranslationObservationKind,
    pub subject: String,
    pub detail: String,
}

pub trait TranslationObserver: Send + Sync + 'static {
    fn observe(&self, observation: &TranslationObservation);
}

#[derive(Clone)]
pub(crate) struct TranslationContext {
    route: TranslationRoute,
    observer: Arc<dyn TranslationObserver>,
}

impl TranslationContext {
    pub(crate) fn new(route: TranslationRoute, observer: Arc<dyn TranslationObserver>) -> Self {
        Self { route, observer }
    }

    pub(crate) fn with_observer(mut self, observer: Arc<dyn TranslationObserver>) -> Self {
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
    observer: Arc<dyn TranslationObserver>,
}

impl TranslationScope {
    pub(crate) fn route(&self) -> TranslationRoute {
        self.route
    }

    pub(crate) fn observe(
        &self,
        kind: TranslationObservationKind,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.observer.observe(&TranslationObservation {
            route: self.route,
            phase: self.phase,
            kind,
            subject: subject.into(),
            detail: detail.into(),
        });
    }

    pub(crate) fn dropped(&self, subject: impl Into<String>, detail: impl Into<String>) {
        self.observe(TranslationObservationKind::Dropped, subject, detail);
    }

    pub(crate) fn adapted(&self, subject: impl Into<String>, detail: impl Into<String>) {
        self.observe(TranslationObservationKind::Adapted, subject, detail);
    }
}

#[derive(Debug, Default)]
pub struct NoopTranslationObserver;

impl TranslationObserver for NoopTranslationObserver {
    fn observe(&self, _observation: &TranslationObservation) {}
}
