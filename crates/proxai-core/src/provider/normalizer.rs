use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};

use crate::observe::{
    NoopObserver, Observer, ProviderObservation, ProviderResponseAdaptation, ProviderResponsePhase,
};
use crate::protocol::ProviderProtocol;
use crate::translation::stream::StreamEvent;

use super::response;

/// Controls whether successful provider responses are repaired to official
/// protocol shapes before translation or identity forwarding.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ProviderCompatibility {
    /// Repair conservative or measured compatibility gaps.
    #[default]
    Compatible,
    /// Preserve successful provider response payloads without compatibility repair.
    Strict,
}

/// Carrier-independent compatibility normalizer for one provider protocol.
#[derive(Clone)]
pub struct ProviderNormalizer {
    protocol: ProviderProtocol,
    compatibility: ProviderCompatibility,
    observer: Arc<dyn Observer>,
}

impl ProviderNormalizer {
    pub fn new(protocol: ProviderProtocol, compatibility: ProviderCompatibility) -> Self {
        Self {
            protocol,
            compatibility,
            observer: Arc::new(NoopObserver),
        }
    }

    pub fn with_observer(mut self, observer: impl Observer + 'static) -> Self {
        self.observer = Arc::new(observer);
        self
    }

    pub fn compatibility(&self) -> ProviderCompatibility {
        self.compatibility
    }

    /// Returns whether this policy can alter structured provider responses.
    ///
    /// Carriers may use this to preserve raw bytes for identity forwarding when
    /// no compatibility repair is configured for the provider protocol.
    pub fn requires_structured_normalization(&self) -> bool {
        self.compatibility == ProviderCompatibility::Compatible
            && matches!(
                self.protocol,
                ProviderProtocol::AnthropicMessages | ProviderProtocol::OpenaiChatCompletions
            )
    }

    pub fn normalize_response(&self, payload: Value) -> Value {
        if !self.requires_structured_normalization() {
            return payload;
        }

        let original = payload.clone();
        let (normalized, adaptation) = match self.protocol {
            ProviderProtocol::AnthropicMessages => (
                response::anthropic_messages::normalize_message_payload(payload),
                ProviderResponseAdaptation::AnthropicMessagesShape,
            ),
            ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
                return payload;
            }
        };
        if normalized != original {
            self.observe(ProviderResponsePhase::NonStreaming, adaptation);
        }
        normalized
    }

    pub fn normalize_stream_event(&self, mut event: StreamEvent) -> StreamEvent {
        if !self.requires_structured_normalization() || event.is_done_sentinel() {
            return event;
        }

        let original_data = event.data.clone();
        let original_event_type = event.event_type.clone();
        let adaptation = match self.protocol {
            ProviderProtocol::AnthropicMessages => {
                event.data =
                    response::anthropic_messages::normalize_stream_event_payload(event.data);
                if let Some(event_type) = event.data.get("type").and_then(Value::as_str) {
                    event.event_type = event_type.to_string();
                }
                ProviderResponseAdaptation::AnthropicMessagesStreamEvent
            }
            ProviderProtocol::OpenaiChatCompletions => {
                event.data =
                    response::openai_chat_completions::normalize_stream_event_payload(event.data);
                ProviderResponseAdaptation::OpenaiChatCompletionsStreamEvent
            }
            ProviderProtocol::OpenaiResponses => return event,
        };
        if event.data != original_data || event.event_type != original_event_type {
            self.observe(ProviderResponsePhase::Streaming, adaptation);
        }
        event
    }

    fn observe(&self, phase: ProviderResponsePhase, adaptation: ProviderResponseAdaptation) {
        self.observer.observe(
            &ProviderObservation::ResponseAdapted {
                protocol: self.protocol,
                phase,
                adaptation,
            }
            .into(),
        );
    }
}

#[cfg(test)]
#[path = "normalizer_tests.rs"]
mod tests;
