//! Carrier-independent observation contract for core request-domain events.
//!
//! Core producers emit typed [`Observation`] values but do not choose logging,
//! diagnostics, capture, metrics, or storage behavior. Downstream composition
//! provides an [`Observer`] implementation and decides which sinks receive each
//! observation.

use derive_more::From;
use strum::Display;

use crate::protocol::{ProviderProtocol, RequestProtocol};

/// Closed set of stable observations emitted by core request-domain logic.
#[derive(Debug, Clone, PartialEq, Eq, From)]
pub enum Observation {
    Ingress(IngressObservation),
    Provider(ProviderObservation),
    Translation(TranslationObservation),
}

/// Compatibility and normalization observations emitted during ingress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngressObservation {
    AnthropicLegacyThinkingBudget { model: String, budget_tokens: u32 },
}

/// A provider compatibility adaptation applied around translation or forwarding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderObservation {
    RequestAdapted {
        protocol: ProviderProtocol,
        adaptation: ProviderRequestAdaptation,
    },
    ResponseAdapted {
        protocol: ProviderProtocol,
        phase: ProviderResponsePhase,
        adaptation: ProviderResponseAdaptation,
    },
}

/// Closed set of provider request compatibility adaptations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRequestAdaptation {
    OpenaiResponsesOutputFieldsRemoved {
        status_removed: usize,
        reasoning_content_removed: usize,
    },
}

/// Structured provider response phase where compatibility repair occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ProviderResponsePhase {
    NonStreaming,
    Streaming,
}

/// Closed set of provider response compatibility repairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ProviderResponseAdaptation {
    AnthropicMessagesShape,
    AnthropicMessagesStreamEvent,
    OpenaiChatCompletionsShape,
    OpenaiChatCompletionsStreamEvent,
    OpenaiResponsesUsageShape,
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
