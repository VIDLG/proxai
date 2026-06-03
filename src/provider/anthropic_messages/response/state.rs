use crate::http_model::UpstreamResponseHead;
use crate::upstream::{UpstreamBodyStreamStats, UpstreamStreamMetrics};
use getset::{CopyGetters, Getters};

use crate::protocol::anthropic::messages::{
    Message, MessageDeltaEvent, MessageStreamEvent, ResponseServiceTier, StopReason,
};

use super::summary::AnthropicResponseSummary;

#[derive(Debug, Clone, Default, Getters, CopyGetters)]
pub(crate) struct AnthropicResponseState {
    #[getset(get = "pub(crate)")]
    id: Option<String>,
    #[getset(get = "pub(crate)")]
    model: Option<String>,
    #[getset(get_copy = "pub(crate)")]
    service_tier: Option<ResponseServiceTier>,
    #[getset(get_copy = "pub(crate)")]
    stop_reason: Option<StopReason>,
    #[getset(get_copy = "pub(crate)")]
    input_tokens: Option<u32>,
    #[getset(get_copy = "pub(crate)")]
    cache_read_input_tokens: Option<u32>,
    #[getset(get_copy = "pub(crate)")]
    cache_creation_input_tokens: Option<u32>,
    #[getset(get_copy = "pub(crate)")]
    output_tokens: Option<u32>,
    pub(crate) summary: AnthropicResponseSummary,
    #[getset(get_copy = "pub(crate)")]
    stream_done: bool,
}

impl AnthropicResponseState {
    pub(crate) fn record_stream_event(&mut self, event: &MessageStreamEvent) {
        self.summary.merge(&AnthropicResponseSummary::from(event));
        match event {
            MessageStreamEvent::MessageStart(event) => self.record_message(&event.message),
            MessageStreamEvent::MessageDelta(event) => self.record_message_delta(event),
            MessageStreamEvent::MessageStop(_) => self.stream_done = true,
            MessageStreamEvent::Ping(_)
            | MessageStreamEvent::ContentBlockStart(_)
            | MessageStreamEvent::ContentBlockDelta(_)
            | MessageStreamEvent::ContentBlockStop(_) => {
                // Content-level stream events are summarized separately by
                // `AnthropicResponseSummary`; this state only tracks
                // message-level fields used by logging and completion checks.
            }
        }
    }

    fn record_message(&mut self, message: &Message) {
        self.id = Some(message.id.clone());
        self.model = Some(message.model.clone());
        self.service_tier = message.usage.service_tier;
        self.stop_reason = message.stop_reason;
        self.input_tokens = Some(message.usage.input_tokens);
        self.cache_read_input_tokens = message.usage.cache_read_input_tokens;
        self.cache_creation_input_tokens = message.usage.cache_creation_input_tokens;
        self.output_tokens = Some(message.usage.output_tokens);
    }

    fn record_message_delta(&mut self, event: &MessageDeltaEvent) {
        self.input_tokens = event.usage.input_tokens;
        self.cache_read_input_tokens = event.usage.cache_read_input_tokens;
        self.cache_creation_input_tokens = event.usage.cache_creation_input_tokens;
        self.output_tokens = Some(event.usage.output_tokens);
        self.stop_reason = event.delta.stop_reason;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AnthropicUpstreamResponseSnapshot {
    pub(crate) head: UpstreamResponseHead,
    pub(crate) metrics: UpstreamStreamMetrics,
    pub(crate) state: AnthropicResponseState,
}

impl AnthropicUpstreamResponseSnapshot {
    pub(crate) fn streaming(
        head: &UpstreamResponseHead,
        stats: UpstreamBodyStreamStats,
        state: AnthropicResponseState,
    ) -> Self {
        Self {
            head: head.clone(),
            metrics: stats.metrics(),
            state,
        }
    }
}
