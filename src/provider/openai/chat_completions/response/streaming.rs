use axum::body::Body;
use axum::http::Response;

use crate::http_support::UpstreamResponseHead;
use crate::http_support::response_with_headers;
use crate::observe::{
    ObserveContext, ProviderStreamOutcome, ProviderStreamOutcomeObserved, ProviderStreamSnapshot,
};
use crate::provider::ProviderStreamingResponsePolicy;
use crate::sse::SseEventScanner;
use crate::upstream::{
    BodyAction, BodyObserver, UpstreamBodyStreamStats, UpstreamStreamError, prepare_response_stream,
};

use super::state::{ChatUpstreamResponseState, ChatUpstreamStreamSnapshot};

pub(crate) fn handle_streaming_response(
    obs: &ObserveContext,
    policy: ProviderStreamingResponsePolicy,
    response: reqwest::Response,
) -> Response<Body> {
    let head = UpstreamResponseHead::from_response(&response, obs.elapsed());
    let body_observer = ChatUpstreamBodyObserver::new((*obs).clone());

    let (outbound_head, body_stream) = prepare_response_stream(
        obs,
        &head,
        policy.read_idle_timeout(),
        response,
        body_observer,
    );

    let (status, headers) = outbound_head.into_parts();
    response_with_headers(status, headers, Body::from_stream(body_stream))
}

struct ChatUpstreamBodyObserver {
    state: ChatUpstreamResponseState,
    sse_scanner: SseEventScanner,
    stream_error: Option<UpstreamStreamError>,
    obs: crate::observe::ObserveContext,
}

impl ChatUpstreamBodyObserver {
    fn new(obs: crate::observe::ObserveContext) -> Self {
        Self {
            state: ChatUpstreamResponseState::default(),
            sse_scanner: SseEventScanner::default(),
            stream_error: None,
            obs,
        }
    }

    fn stream_snapshot(
        &self,
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
    ) -> ChatUpstreamStreamSnapshot {
        ChatUpstreamStreamSnapshot {
            head: head.clone(),
            metrics: stats.metrics(),
            state: self.state.clone(),
        }
    }
}

impl BodyObserver for ChatUpstreamBodyObserver {
    fn on_chunk(&mut self, chunk: &[u8]) -> BodyAction {
        let events = self.sse_scanner.scan(chunk);
        self.state.observe_events(&events);
        BodyAction::Continue
    }

    fn on_stream_error(&mut self, error: &reqwest::Error) {
        self.stream_error = Some(UpstreamStreamError::Stream {
            message: error.to_string(),
        });
    }

    fn on_stream_finished(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats) {
        let snapshot = self.stream_snapshot(head, stats);
        let outcome = if let Some(ref error) = self.stream_error {
            ProviderStreamOutcome::Error(error)
        } else if self.state.stream_done {
            ProviderStreamOutcome::Completed
        } else {
            ProviderStreamOutcome::Closed
        };
        self.obs
            .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                snapshot: ProviderStreamSnapshot::OpenaiChatCompletions(&snapshot),
                outcome,
            });
    }
}

#[cfg(test)]
#[path = "handle_tests.rs"]
mod tests;
