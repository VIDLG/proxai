use crate::protocol::openai_responses::ResponseProjection;
use crate::protocol::ErrorObject;

use crate::http_model::UpstreamResponseHead;
use crate::upstream::{UpstreamStreamError, UpstreamStreamMetrics};

use super::limits::ResponseLimits;
use super::observed::{ObservedState, ObservedUpdate};
use super::summary::ResponseSummary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResponseSnapshotKind {
    Created,
    InProgress,
    Completed,
    Failed,
    Incomplete,
    Queued,
}

#[derive(Debug, Clone)]
pub(crate) struct ResponseSnapshot {
    pub(crate) kind: ResponseSnapshotKind,
    pub(crate) projection: ResponseProjection,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ResponsesUpstreamState {
    /// The latest response snapshot received from upstream `response.*` events.
    pub(crate) latest_snapshot: Option<ResponseSnapshot>,

    /// Best-effort observed stream state reconstructed from incremental
    /// stream events when a full response snapshot is absent or incomplete for
    /// diagnostics/logging.
    ///
    /// This is fallback-only state: summaries should still be rebuilt from the
    /// latest upstream `response.*` snapshot when available. It intentionally lives for the full tracker
    /// lifetime and is not reset by later snapshots, so mid-stream
    /// observations remain available for timeout / unfinished-stream
    /// diagnostics.
    observed: ObservedState,
    pub(crate) sequence_number: Option<u64>,
}

impl ResponsesUpstreamState {
    pub(crate) fn set_snapshot(
        &mut self,
        kind: ResponseSnapshotKind,
        projection: ResponseProjection,
    ) {
        self.latest_snapshot = Some(ResponseSnapshot { kind, projection });
    }

    pub(super) fn record_sequence_number(&mut self, sequence_number: u64) {
        self.sequence_number = Some(sequence_number);
    }

    pub(crate) fn record_observed_error(&mut self, error: ErrorObject) {
        self.observed.record_error(error);
    }

    pub(super) fn apply_observed_update(&mut self, update: &ObservedUpdate) {
        self.observed.apply(update);
    }

    pub(crate) fn fallback_summary(&self) -> ResponseSummary {
        ResponseSummary::from(&self.observed)
    }

    pub(crate) fn observed_error(&self) -> Option<&ErrorObject> {
        self.observed.error()
    }

    pub(crate) fn primary_summary(&self) -> Option<ResponseSummary> {
        let snapshot = self.latest_snapshot.as_ref()?;
        let summary = ResponseSummary::from(&snapshot.projection);
        (!summary.is_empty()).then_some(summary)
    }

    pub(crate) fn effective_summary(&self) -> ResponseSummary {
        self.primary_summary()
            .unwrap_or_else(|| self.fallback_summary())
    }

    pub(crate) fn effective_error(&self) -> Option<&ErrorObject> {
        if let Some(snapshot) = self.latest_snapshot.as_ref() {
            if let Some(error) = snapshot.projection.error.as_ref() {
                return Some(error);
            }
        }

        self.observed_error()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ResponsesUpstreamMetadata {
    pub(crate) limits: ResponseLimits,
}

impl ResponsesUpstreamMetadata {
    pub(crate) fn from_head(head: &UpstreamResponseHead) -> Self {
        Self {
            limits: ResponseLimits::from_headers(&head.headers),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResponsesUpstreamStreamSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: ResponsesUpstreamState,
    pub(crate) metadata: ResponsesUpstreamMetadata,
}

#[derive(Debug, Clone)]
pub(crate) enum ResponsesUpstreamEvent {
    Headers {
        head: UpstreamResponseHead,
    },
    Completed {
        snapshot: Box<ResponsesUpstreamStreamSnapshot>,
    },
    Closed {
        snapshot: Box<ResponsesUpstreamStreamSnapshot>,
    },
    Error {
        snapshot: Box<ResponsesUpstreamStreamSnapshot>,
        error: UpstreamStreamError,
    },
}
