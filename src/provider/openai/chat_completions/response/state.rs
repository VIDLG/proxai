use crate::upstream::{UpstreamResponseHead, UpstreamStreamMetrics};

use super::observed::{ChatResponseObservation, ObservedChatState, ObservedChatUpdate};
use super::summary::ChatResponseSummary;

/// Whether the latest parsed response shape is only an in-progress observation
/// or can be used as the terminal response view for logging/summary purposes.
#[derive(Debug, Clone)]
enum ChatResponseStage {
    Partial(ChatResponseObservation),
    Terminal(ChatResponseObservation),
}

impl ChatResponseStage {
    fn response(&self) -> &ChatResponseObservation {
        match self {
            Self::Partial(response) | Self::Terminal(response) => response,
        }
    }

    fn terminal_response(&self) -> Option<&ChatResponseObservation> {
        match self {
            Self::Terminal(response) => Some(response),
            Self::Partial(_) => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ChatUpstreamResponseState {
    response: Option<ChatResponseStage>,
    observed: ObservedChatState,
    pub(crate) stream_done: bool,
    is_sse: bool,
}

impl ChatUpstreamResponseState {
    pub(crate) fn new(is_sse: bool) -> Self {
        Self {
            is_sse,
            ..Self::default()
        }
    }

    pub(crate) fn is_sse(&self) -> bool {
        self.is_sse
    }

    pub(crate) fn eof_is_complete(&self) -> bool {
        !self.is_sse || self.stream_done
    }

    pub(crate) fn terminal_response(&self) -> Option<&ChatResponseObservation> {
        self.response
            .as_ref()
            .and_then(ChatResponseStage::terminal_response)
    }

    pub(crate) fn record_terminal(&mut self, response: ChatResponseObservation) {
        self.response = Some(ChatResponseStage::Terminal(response));
    }

    pub(crate) fn record_partial(&mut self, response: ChatResponseObservation) {
        self.response = Some(ChatResponseStage::Partial(response));
    }

    pub(crate) fn apply_observed_update(&mut self, update: &ObservedChatUpdate) {
        self.observed.apply(update);
    }

    pub(crate) fn fallback_summary(&self) -> ChatResponseSummary {
        self.observed.fallback_summary()
    }

    pub(crate) fn primary_summary(&self) -> Option<ChatResponseSummary> {
        let terminal_response = self.terminal_response()?;
        let summary = terminal_response.summary();
        (!summary.is_empty()).then_some(summary)
    }

    pub(crate) fn effective_summary(&self) -> ChatResponseSummary {
        self.primary_summary()
            .unwrap_or_else(|| self.fallback_summary())
    }

    pub(crate) fn effective_response(&self) -> Option<&ChatResponseObservation> {
        self.response.as_ref().map(ChatResponseStage::response)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChatUpstreamStreamSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: ChatUpstreamResponseState,
}
