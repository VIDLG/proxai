//! Carrier-independent observation contract for core request-domain events.
//!
//! Core producers emit typed [`Observation`] values but do not choose logging,
//! diagnostics, capture, metrics, or storage behavior. Downstream composition
//! provides an [`Observer`] implementation and decides which sinks receive each
//! observation.

use derive_more::From;

use crate::protocol::{ProviderProtocol, RequestProtocol};

/// Closed set of stable observations emitted by core request-domain logic.
#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum Observation {
    Ingress(IngressObservation),
    Translation(TranslationObservation),
}

/// Compatibility and normalization observations emitted during ingress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngressObservation {
    AnthropicLegacyThinkingBudget { model: String, budget_tokens: u32 },
}

/// Core pipeline phase in which a translation observation was emitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationPhase {
    Request,
    NonStreamingResponse,
    StreamingResponse,
}

/// Whether translation preserved behavior by adaptation or lost source detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationObservationKind {
    Dropped,
    Adapted,
}

/// A legal, non-fatal protocol translation decision worth observing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationObservation {
    pub request_protocol: RequestProtocol,
    pub provider_protocol: ProviderProtocol,
    pub phase: TranslationPhase,
    pub kind: TranslationObservationKind,
    pub subject: String,
    pub detail: String,
}

/// Receives typed core observations without prescribing concrete sink behavior.
pub trait Observer: Send + Sync {
    fn observe(&self, observation: &Observation);
}

/// Default observer for callers that do not need observation output.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopObserver;

impl Observer for NoopObserver {
    fn observe(&self, _observation: &Observation) {}
}
