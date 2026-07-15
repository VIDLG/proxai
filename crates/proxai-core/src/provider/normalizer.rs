use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};

use crate::observe::{
    Observer, ProviderObservation, ProviderResponseAdaptation, ProviderResponsePhase,
};
use crate::protocol::ProviderProtocol;
use crate::translation::stream::StreamEvent;

use super::{ProviderBehavior, response};

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

/// Returns whether this provider behavior can alter structured responses.
///
/// Carriers may use this to preserve raw bytes for identity forwarding when no
/// compatibility repair is configured for the provider protocol.
pub fn requires_structured_normalization(behavior: ProviderBehavior) -> bool {
    behavior.compatibility() == ProviderCompatibility::Compatible
        && matches!(
            behavior.protocol(),
            ProviderProtocol::AnthropicMessages | ProviderProtocol::OpenaiChatCompletions
        )
}

/// Normalize one non-streaming provider response payload when the configured
/// provider behavior requires it.
pub fn normalize_provider_response(
    behavior: ProviderBehavior,
    payload: Value,
    observer: &dyn Observer,
) -> Value {
    if !requires_structured_normalization(behavior) {
        return payload;
    }

    let original = payload.clone();
    let (normalized, adaptation) = match behavior.protocol() {
        ProviderProtocol::AnthropicMessages => (
            response::anthropic_messages::normalize_message_payload(payload),
            ProviderResponseAdaptation::AnthropicMessagesShape,
        ),
        ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
            return payload;
        }
    };
    if normalized != original {
        observer.observe(
            &ProviderObservation::ResponseAdapted {
                protocol: behavior.protocol(),
                phase: ProviderResponsePhase::NonStreaming,
                adaptation,
            }
            .into(),
        );
    }
    normalized
}

/// Normalize one provider stream event when the configured provider behavior
/// requires it.
pub fn normalize_provider_stream_event(
    behavior: ProviderBehavior,
    mut event: StreamEvent,
    observer: &dyn Observer,
) -> StreamEvent {
    if !requires_structured_normalization(behavior) || event.is_done_sentinel() {
        return event;
    }

    let original_data = event.data.clone();
    let original_event_type = event.event_type.clone();
    let adaptation = match behavior.protocol() {
        ProviderProtocol::AnthropicMessages => {
            event.data = response::anthropic_messages::normalize_stream_event_payload(event.data);
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
        observer.observe(
            &ProviderObservation::ResponseAdapted {
                protocol: behavior.protocol(),
                phase: ProviderResponsePhase::Streaming,
                adaptation,
            }
            .into(),
        );
    }
    event
}

#[cfg(test)]
#[path = "normalizer_tests.rs"]
mod tests;
