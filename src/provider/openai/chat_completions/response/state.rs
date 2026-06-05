use crate::http_support::UpstreamResponseHead;
use crate::protocol::openai::chat_completions::{
    ChatStreamResponseProjection, CreateChatCompletionStreamResponse,
};
use crate::sse::SseEvent;
use crate::upstream::UpstreamStreamMetrics;

use super::observed::{
    ChatResponseObservation, ObservedChatState, ObservedChatUpdate,
    observed_updates_from_stream_projection,
};
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
}

impl ChatUpstreamResponseState {
    pub(crate) fn observe_events(&mut self, events: &[SseEvent]) {
        for event in events {
            if event.is_done_sentinel() {
                self.stream_done = true;
                continue;
            }
            let Ok(response) =
                serde_json::from_str::<CreateChatCompletionStreamResponse>(&event.data)
            else {
                continue;
            };
            let projection = ChatStreamResponseProjection::from(response);
            for update in observed_updates_from_stream_projection(&projection) {
                self.apply_observed_update(&update);
            }
            let observed = ChatResponseObservation::StreamChunk(projection);
            if observed.has_finish_reason() {
                self.record_terminal(observed);
            } else {
                self.record_partial(observed);
            }
        }
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

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
