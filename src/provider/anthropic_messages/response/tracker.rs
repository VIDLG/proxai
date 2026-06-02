use axum::http::HeaderMap;

use crate::protocol::anthropic::messages::{Message, MessageStreamEvent};
use crate::sse::SseEventScanner;
use crate::upstream::UpstreamResponseHead;
use std::time::Duration;

use super::normalize::{normalize_message_payload, normalize_stream_event_payload};
use super::state::AnthropicResponseState;

#[derive(Debug, Default)]
pub(crate) struct AnthropicResponseTracker {
    pub(crate) state: AnthropicResponseState,
    sse_scanner: SseEventScanner,
    is_sse: bool,
    json_body: Vec<u8>,
}

impl AnthropicResponseTracker {
    pub(crate) fn from_headers(headers: &HeaderMap) -> Self {
        let head = UpstreamResponseHead::from_headers(
            axum::http::StatusCode::OK,
            headers,
            Duration::default(),
        );
        Self {
            is_sse: head.is_sse(),
            ..Self::default()
        }
    }

    pub(crate) fn scan_bytes(&mut self, chunk: &[u8]) {
        if self.is_sse {
            self.scan_sse_bytes(chunk);
        } else {
            self.json_body.extend_from_slice(chunk);
        }
    }

    pub(crate) fn finish(&mut self) {
        if self.is_sse || self.json_body.is_empty() {
            return;
        }
        let Ok(payload) = serde_json::from_slice::<serde_json::Value>(&self.json_body) else {
            return;
        };
        let Ok(message) = serde_json::from_value::<Message>(normalize_message_payload(payload))
        else {
            return;
        };
        self.state.record_message(message);
        self.json_body.clear();
    }

    fn scan_sse_bytes(&mut self, chunk: &[u8]) {
        for event in self.sse_scanner.scan(chunk) {
            let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.data) else {
                continue;
            };
            let Ok(event) = serde_json::from_value::<MessageStreamEvent>(
                normalize_stream_event_payload(payload),
            ) else {
                continue;
            };
            self.state.record_stream_event(&event);
        }
    }
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
