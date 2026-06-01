use getset::{CopyGetters, Getters};

use crate::protocol::anthropic::messages::{
    Message, MessageStreamEvent, ResponseServiceTier, StopReason,
};

use super::summary::AnthropicResponseSummary;

#[derive(Debug, Clone, Default, Getters, CopyGetters)]
pub(crate) struct AnthropicResponseProjection {
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
}

impl AnthropicResponseProjection {
    fn record_stream_event(&mut self, event: &MessageStreamEvent) {
        match event {
            MessageStreamEvent::MessageStart(event) => {
                *self = Self::from(&event.message);
            }
            MessageStreamEvent::MessageDelta(event) => {
                self.input_tokens = event.usage.input_tokens;
                self.cache_read_input_tokens = event.usage.cache_read_input_tokens;
                self.cache_creation_input_tokens = event.usage.cache_creation_input_tokens;
                self.output_tokens = Some(event.usage.output_tokens);
                self.stop_reason = event.delta.stop_reason;
            }
            MessageStreamEvent::MessageStop(_) => {}
            MessageStreamEvent::Ping(_)
            | MessageStreamEvent::ContentBlockStart(_)
            | MessageStreamEvent::ContentBlockDelta(_)
            | MessageStreamEvent::ContentBlockStop(_) => {
                // Content-level stream events are summarized separately by
                // `AnthropicResponseSummary`; this projection only tracks
                // message-level fields used by logging and completion checks.
            }
        }
    }
}

impl From<&Message> for AnthropicResponseProjection {
    fn from(message: &Message) -> Self {
        Self {
            id: Some(message.id.clone()),
            model: Some(message.model.clone()),
            service_tier: message.usage.service_tier,
            stop_reason: message.stop_reason,
            input_tokens: Some(message.usage.input_tokens),
            cache_read_input_tokens: message.usage.cache_read_input_tokens,
            cache_creation_input_tokens: message.usage.cache_creation_input_tokens,
            output_tokens: Some(message.usage.output_tokens),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AnthropicResponseState {
    pub(crate) projection: AnthropicResponseProjection,
    pub(crate) summary: AnthropicResponseSummary,
    stream_done: bool,
}

impl AnthropicResponseState {
    pub(crate) fn record_message(&mut self, message: Message) {
        self.summary
            .merge(&AnthropicResponseSummary::from(&message));
        self.projection = AnthropicResponseProjection::from(&message);
    }

    pub(crate) fn record_stream_event(&mut self, event: &MessageStreamEvent) {
        self.summary.merge(&AnthropicResponseSummary::from(event));
        if matches!(event, MessageStreamEvent::MessageStop(_)) {
            self.stream_done = true;
        }
        self.projection.record_stream_event(event);
    }

    pub(crate) fn stream_done(&self) -> bool {
        self.stream_done
    }
}
