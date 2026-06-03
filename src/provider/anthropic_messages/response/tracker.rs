use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::sse::SseEventScanner;

use super::normalize::normalize_stream_event_payload;
use super::state::AnthropicResponseState;

#[derive(Debug, Default)]
pub(crate) struct AnthropicResponseTracker {
    pub(crate) state: AnthropicResponseState,
    sse_scanner: SseEventScanner,
}

impl AnthropicResponseTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn scan_bytes(&mut self, chunk: &[u8]) {
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
