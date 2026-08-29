//! Inbound-side streaming lifecycle shared by every translator rooted at
//! `anthropic_messages`.
//!
//! This module owns the source-protocol phase ordering, state access helpers,
//! inbound event allowlisting, and the common tracker for target-representable
//! output progress while a semantic stream is active.

use delegate::delegate;
use serde_json::Value;

use crate::json::deserialize_value;
use crate::protocol::anthropic::messages::{MessageStreamEvent, StopReason};
use crate::translation::stream::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, StreamEnd, StreamIdentity,
    StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

/// Whether Anthropic may legitimately terminate before emitting any content
/// block that a target protocol can represent.
///
/// `model_context_window_exceeded` is observed as a pre-generation terminal:
/// the stream contains `message_start`, then a terminal `message_delta` with
/// zero output tokens, and no content blocks. Other stop reasons remain subject
/// to the strict non-empty-output invariant.
pub(crate) fn stop_reason_allows_empty_output(stop_reason: StopReason) -> bool {
    stop_reason == StopReason::ModelContextWindowExceeded
}

#[derive(Debug)]
pub(crate) struct AnthropicInboundLifecycle<S> {
    inner: InboundStreamLifecycle<S, StreamingPhase<S>>,
}

impl<S> Default for AnthropicInboundLifecycle<S> {
    fn default() -> Self {
        Self {
            inner: InboundStreamLifecycle::default(),
        }
    }
}

impl<S> AnthropicInboundLifecycle<S> {
    delegate! {
        to self.inner {
            #[call(receive_terminal)]
            pub(crate) fn receive_terminal_delta(&mut self, phase: StreamingPhase<S>);
            pub(crate) fn stop(&mut self);
        }
    }

    pub(crate) fn parse_stream_event(
        &self,
        payload: Value,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let parsed =
            deserialize_value::<MessageStreamEvent>(&payload, "Anthropic Messages stream event")?;
        if matches!(parsed, MessageStreamEvent::Ping(_)) {
            return Ok(parsed);
        }

        match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                if !matches!(parsed, MessageStreamEvent::MessageStart(_)) {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted semantic event before message_start".to_string(),
                    ));
                }
            }
            InboundStreamLifecyclePhase::Streaming => {
                if matches!(parsed, MessageStreamEvent::MessageStop(_)) {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted message_stop before terminal message_delta"
                            .to_string(),
                    ));
                }
            }
            InboundStreamLifecyclePhase::Terminal => {
                if !matches!(parsed, MessageStreamEvent::MessageStop(_)) {
                    return Err(StreamTranslationError::Semantic(
                        "Anthropic stream emitted semantic event after terminal message_delta before message_stop"
                            .to_string(),
                    ));
                }
            }
            InboundStreamLifecyclePhase::Stopped => {
                return Err(StreamTranslationError::Semantic(
                    "Anthropic stream emitted semantic event after message_stop".to_string(),
                ));
            }
        }
        Ok(parsed)
    }

    pub(crate) fn begin_message_stream(
        &mut self,
        identity: StreamIdentity,
        state: S,
    ) -> StreamTranslationResult<()> {
        if !self.inner.is_waiting() {
            return Err(StreamTranslationError::Semantic(
                "Anthropic stream emitted duplicate message_start".to_string(),
            ));
        }
        self.inner.begin_streaming(identity, state);
        Ok(())
    }

    pub(crate) fn stream_identity(&self) -> StreamTranslationResult<&StreamIdentity> {
        self.inner.require_identity(|| {
            StreamTranslationError::Semantic(
                "Anthropic stream identity is not initialized before message_start".to_string(),
            )
        })
    }

    pub(crate) fn streaming_state(&self) -> StreamTranslationResult<&S> {
        self.inner
            .streaming_phase()
            .map(StreamingPhase::state)
            .ok_or_else(|| {
                StreamTranslationError::Semantic(
                    "Anthropic stream active content event occurred outside streaming state"
                        .to_string(),
                )
            })
    }

    pub(crate) fn streaming_phase_mut(
        &mut self,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        self.inner
            .require_streaming_phase_mut("Anthropic", "active content event")
    }

    pub(crate) fn streaming_state_mut(&mut self) -> StreamTranslationResult<&mut S> {
        Ok(self.streaming_phase_mut()?.state_mut())
    }

    pub(crate) fn take_streaming_phase(&mut self) -> StreamTranslationResult<StreamingPhase<S>> {
        self.inner.take_streaming_phase(|| {
            StreamTranslationError::Semantic(
                "Anthropic stream terminal event occurred outside streaming state".to_string(),
            )
        })
    }

    pub(crate) fn take_terminal_phase(&mut self) -> StreamTranslationResult<StreamingPhase<S>> {
        self.inner.take_terminal(|| {
            StreamTranslationError::Semantic(
                "Anthropic stream message_stop occurred before terminal message_delta".to_string(),
            )
        })
    }

    /// Validate that the semantic Anthropic stream reached `message_stop`
    /// before its carrier ended.
    pub(crate) fn finish_stream(&self, end: StreamEnd) -> StreamTranslationResult<()> {
        let message = match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                format!("Anthropic stream reached {end} before message_start")
            }
            InboundStreamLifecyclePhase::Streaming => {
                let phase = self
                    .inner
                    .streaming_phase()
                    .expect("streaming phase exists");
                if phase.emitted_any() {
                    format!("Anthropic stream reached {end} before terminal message_delta")
                } else {
                    "Anthropic stream completed without target-representable content, thinking, refusal, or tool_use blocks"
                        .to_string()
                }
            }
            InboundStreamLifecyclePhase::Terminal => {
                format!(
                    "Anthropic stream reached {end} after terminal message_delta but before message_stop"
                )
            }
            InboundStreamLifecyclePhase::Stopped => return Ok(()),
        };
        Err(StreamTranslationError::Semantic(message))
    }
}
