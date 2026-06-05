use axum::body::Body;
use axum::http::Response;

use crate::config::ProviderCompatibility;
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

use super::normalize;
use super::state::{AnthropicResponseState, AnthropicUpstreamResponseSnapshot};

pub(crate) fn handle_streaming_response(
    obs: &ObserveContext,
    policy: ProviderStreamingResponsePolicy,
    compatibility: ProviderCompatibility,
    response: reqwest::Response,
) -> Response<Body> {
    let head = UpstreamResponseHead::from_response(&response, obs.elapsed());
    let body_observer = AnthropicSseObserver::new((*obs).clone());
    let (outbound_head, body_stream) = prepare_response_stream(
        obs,
        &head,
        policy.read_idle_timeout(),
        response,
        body_observer,
    );

    if matches!(
        compatibility,
        crate::config::ProviderCompatibility::AnthropicCompatible
    ) {
        let (status, headers) = outbound_head.clone().into_parts();
        return response_with_headers(
            status,
            headers,
            Body::from_stream(normalize::normalize_sse_stream(body_stream)),
        );
    }

    let (status, headers) = outbound_head.into_parts();
    response_with_headers(status, headers, Body::from_stream(body_stream))
}

/// Minimal SSE observer for Anthropic Messages streaming responses.
pub(super) struct AnthropicSseObserver {
    state: AnthropicResponseState,
    stream_error: Option<UpstreamStreamError>,
    sse_scanner: SseEventScanner,
    obs: crate::observe::ObserveContext,
}

impl AnthropicSseObserver {
    pub(super) fn new(obs: crate::observe::ObserveContext) -> Self {
        Self {
            state: AnthropicResponseState::default(),
            stream_error: None,
            sse_scanner: SseEventScanner::default(),
            obs,
        }
    }

    fn stream_snapshot(
        &self,
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
    ) -> AnthropicUpstreamResponseSnapshot {
        AnthropicUpstreamResponseSnapshot::streaming(head, stats, self.state.clone())
    }
}

impl BodyObserver for AnthropicSseObserver {
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
        } else if self.state.stream_done() {
            ProviderStreamOutcome::Completed
        } else {
            ProviderStreamOutcome::Closed
        };
        self.obs
            .observe_provider_stream_outcome(ProviderStreamOutcomeObserved {
                snapshot: ProviderStreamSnapshot::AnthropicMessages(&snapshot),
                outcome,
            });
    }
}

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
