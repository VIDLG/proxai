//! Inbound-side streaming lifecycle shared by every translator rooted at
//! `anthropic_messages`.
//!
//! This module owns the source-protocol phase ordering, state access helpers,
//! inbound event allowlisting, and the common tracker for target-representable
//! output progress while a semantic stream is active.

use serde_json::Value;

use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::translation::streaming::{
    SseStreamEnd, StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

#[derive(Debug, Default)]
pub(crate) enum AnthropicInboundLifecycle<S> {
    #[default]
    WaitingForMessageStart,
    Streaming(StreamingPhase<S>),
    ReceivedTerminalDelta(StreamingPhase<S>),
    Stopped,
}

impl<S> AnthropicInboundLifecycle<S> {
    pub(crate) fn begin_streaming(&mut self, state: S) {
        *self = Self::Streaming(StreamingPhase::new(state));
    }

    pub(crate) fn receive_terminal_delta(&mut self, phase: StreamingPhase<S>) {
        *self = Self::ReceivedTerminalDelta(phase);
    }

    pub(crate) fn stop(&mut self) {
        *self = Self::Stopped;
    }

    pub(crate) fn is_waiting_for_message_start(&self) -> bool {
        matches!(self, Self::WaitingForMessageStart)
    }

    pub(crate) fn ensure_event_allowed(
        &self,
        event: &MessageStreamEvent,
    ) -> StreamTranslationResult<()> {
        if matches!(event, MessageStreamEvent::Ping(_)) {
            return Ok(());
        }

        match self {
            Self::WaitingForMessageStart => {
                if matches!(event, MessageStreamEvent::MessageStart(_)) {
                    Ok(())
                } else {
                    Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted semantic event before message_start".to_string(),
                    ))
                }
            }
            Self::Streaming(_) => {
                if matches!(event, MessageStreamEvent::MessageStop(_)) {
                    Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_stop before terminal message_delta"
                            .to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            Self::ReceivedTerminalDelta(_) => {
                if matches!(event, MessageStreamEvent::MessageStop(_)) {
                    Ok(())
                } else {
                    Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted semantic event after terminal message_delta before message_stop"
                            .to_string(),
                    ))
                }
            }
            Self::Stopped => Err(StreamTranslationError::Semantic(
                "Anthropic stream emitted semantic event after message_stop".to_string(),
            )),
        }
    }

    pub(crate) fn streaming_state(&self) -> StreamTranslationResult<&S> {
        match self {
            Self::Streaming(phase) => Ok(phase.state()),
            _ => Err(StreamTranslationError::Semantic(
                "Anthropic stream active content event occurred outside streaming state"
                    .to_string(),
            )),
        }
    }

    pub(crate) fn streaming_phase_mut(
        &mut self,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        match self {
            Self::Streaming(phase) => Ok(phase),
            _ => Err(StreamTranslationError::Semantic(
                "Anthropic stream active content event occurred outside streaming state"
                    .to_string(),
            )),
        }
    }

    pub(crate) fn streaming_state_mut(&mut self) -> StreamTranslationResult<&mut S> {
        Ok(self.streaming_phase_mut()?.state_mut())
    }

    pub(crate) fn take_streaming_phase(&mut self) -> StreamTranslationResult<StreamingPhase<S>> {
        match std::mem::take(self) {
            Self::Streaming(phase) => Ok(phase),
            other => {
                *self = other;
                Err(StreamTranslationError::Semantic(
                    "Anthropic stream terminal event occurred outside streaming state".to_string(),
                ))
            }
        }
    }

    pub(crate) fn take_terminal_phase(&mut self) -> StreamTranslationResult<StreamingPhase<S>> {
        match std::mem::take(self) {
            Self::ReceivedTerminalDelta(phase) => Ok(phase),
            other => {
                *self = other;
                Err(StreamTranslationError::Semantic(
                    "Anthropic stream message_stop occurred before terminal message_delta"
                        .to_string(),
                ))
            }
        }
    }

    pub(crate) fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped)
    }

    pub(crate) fn unexpected_stream_end_error(
        &self,
        end: SseStreamEnd,
        target_protocol_label: &'static str,
    ) -> StreamTranslationError {
        let end_label = match end {
            SseStreamEnd::DoneSentinel => "[DONE]",
            SseStreamEnd::Eof => "EOF",
        };

        let message = match self {
            Self::WaitingForMessageStart => {
                format!("Anthropic stream reached {end_label} before message_start")
            }
            Self::ReceivedTerminalDelta(_) => {
                format!(
                    "Anthropic stream reached {end_label} after terminal message_delta but before message_stop"
                )
            }
            Self::Streaming(phase) if phase.emitted_any() => {
                format!("Anthropic stream reached {end_label} before terminal message_delta")
            }
            Self::Streaming(_) => format!(
                "Anthropic stream completed without {target_protocol_label}-representable content, thinking, refusal, or tool_use blocks"
            ),
            Self::Stopped => String::new(),
        };
        StreamTranslationError::Semantic(message)
    }
}

pub(crate) fn ensure_anthropic_stream_event(payload: &Value) -> StreamTranslationResult<()> {
    match payload.get("type").and_then(Value::as_str) {
        Some(
            "ping"
            | "message_start"
            | "content_block_start"
            | "content_block_delta"
            | "content_block_stop"
            | "message_delta"
            | "message_stop",
        ) => Ok(()),
        Some(event_type) => Err(StreamTranslationError::Semantic(format!(
            "Anthropic stream emitted unsupported event type `{event_type}`"
        ))),
        None => Err(StreamTranslationError::Semantic(
            "Anthropic stream event is missing `type`".to_string(),
        )),
    }
}
