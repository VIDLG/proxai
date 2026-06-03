use axum::http::HeaderMap;
use std::time::Duration;

use crate::http_support::UpstreamResponseHead;
use crate::protocol::openai::chat_completions::{
    ChatResponseProjection, ChatStreamResponseProjection, CreateChatCompletionResponse,
    CreateChatCompletionStreamResponse,
};
use crate::sse::SseEventScanner;

use super::observed::{ChatResponseObservation, observed_updates_from_stream_projection};
use super::state::ChatUpstreamResponseState;

#[derive(Debug, Default)]
pub(crate) struct ChatUpstreamResponseTracker {
    pub(crate) state: ChatUpstreamResponseState,
    sse_scanner: SseEventScanner,
    pub(crate) json_body: Vec<u8>,
}

impl ChatUpstreamResponseTracker {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let head = UpstreamResponseHead::from_headers(
            axum::http::StatusCode::OK,
            headers,
            Duration::default(),
        );
        let is_sse = head.is_sse();
        Self {
            state: ChatUpstreamResponseState::new(is_sse),
            ..Self::default()
        }
    }

    pub(crate) fn scan_bytes(&mut self, chunk: &[u8]) {
        if self.state.is_sse() {
            self.scan_sse_bytes(chunk);
        } else {
            self.json_body.extend_from_slice(chunk);
            self.finish();
        }
    }

    pub(crate) fn finish(&mut self) {
        if self.state.is_sse() || self.json_body.is_empty() {
            return;
        }
        let Ok(response) = serde_json::from_slice::<CreateChatCompletionResponse>(&self.json_body)
        else {
            return;
        };
        let projection = ChatResponseProjection::from(response);
        self.state
            .record_terminal(ChatResponseObservation::NonStream(projection));
        self.json_body.clear();
    }

    fn scan_sse_bytes(&mut self, chunk: &[u8]) {
        for event in self.sse_scanner.scan(chunk) {
            if event.is_done_sentinel() {
                self.state.stream_done = true;
                continue;
            }
            let Ok(response) =
                serde_json::from_str::<CreateChatCompletionStreamResponse>(&event.data)
            else {
                continue;
            };
            let projection = ChatStreamResponseProjection::from(response);
            for update in observed_updates_from_stream_projection(&projection) {
                self.state.apply_observed_update(&update);
            }
            let observed = ChatResponseObservation::StreamChunk(projection);
            if observed.has_finish_reason() {
                self.state.record_terminal(observed);
            } else {
                self.state.record_partial(observed);
            }
        }
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
