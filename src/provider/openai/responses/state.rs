use axum::http::HeaderMap;

use crate::protocol::openai_responses::ResponseProjection;
use crate::protocol::ErrorObject;

use crate::provider::UpstreamResponseError;
use crate::upstream::{ContentType, UpstreamResponseHead, UpstreamStreamMetrics};

use super::limits::{CodexLimits, RateLimit};
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
    is_sse: bool,

    /// The latest response snapshot received from upstream `response.*` events.
    pub(crate) snapshot: Option<ResponseSnapshot>,

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
    pub(crate) rate_limit: RateLimit,
    pub(crate) codex_limits: CodexLimits,
}

impl ResponsesUpstreamState {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            is_sse: headers
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| ContentType::try_from(value).ok())
                .is_some_and(|content_type| content_type.is_sse()),
            rate_limit: RateLimit::from_headers(headers),
            codex_limits: CodexLimits::from_headers(headers),
            ..Self::default()
        }
    }

    /// Non-SSE responses are complete when the HTTP body reaches EOF. SSE
    /// responses need an explicit terminal event; EOF without one is a closed
    /// or incomplete stream.
    pub(crate) fn eof_is_complete(&self, saw_terminal_event: bool) -> bool {
        !self.is_sse || saw_terminal_event
    }

    pub(crate) fn set_snapshot(
        &mut self,
        kind: ResponseSnapshotKind,
        projection: ResponseProjection,
    ) {
        self.snapshot = Some(ResponseSnapshot { kind, projection });
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

    pub(crate) fn observed_summary(&self) -> ResponseSummary {
        ResponseSummary::from(&self.observed)
    }

    pub(crate) fn observed_error(&self) -> Option<&ErrorObject> {
        self.observed.error()
    }

    pub(crate) fn effective_summary(&self) -> ResponseSummary {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return self.observed_summary();
        };

        let snapshot_summary = ResponseSummary::from(&snapshot.projection);
        if snapshot_summary.is_empty() {
            self.observed_summary()
        } else {
            snapshot_summary
        }
    }

    pub(crate) fn effective_error(&self) -> Option<&ErrorObject> {
        if let Some(snapshot) = self.snapshot.as_ref() {
            if let Some(error) = snapshot.projection.error.as_ref() {
                return Some(error);
            }
        }

        self.observed_error()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ResponsesUpstreamStreamSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: ResponsesUpstreamState,
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
        error: UpstreamResponseError,
    },
}
