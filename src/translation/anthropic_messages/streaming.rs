//! Inbound-side streaming lifecycle shared by every translator rooted at
//! `anthropic_messages`.
//!
//! This module owns the source-protocol phase ordering, state access helpers,
//! inbound event allowlisting, and the common tracker for target-representable
//! output progress while a semantic stream is active.

use delegate::delegate;
use serde_json::Value;

use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::sse::SseEvent;
use crate::translation::streaming::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, RequireStreamingPhaseContext,
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

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
            pub(crate) fn is_stopped(&self) -> bool;
        }
    }

    pub(crate) fn parse_allowed_stream_event(
        &self,
        event: SseEvent,
    ) -> StreamTranslationResult<MessageStreamEvent> {
        let payload = event.payload_with_type()?;
        match payload.get("type").and_then(Value::as_str) {
            Some(
                "ping"
                | "message_start"
                | "content_block_start"
                | "content_block_delta"
                | "content_block_stop"
                | "message_delta"
                | "message_stop",
            ) => {}
            Some(event_type) => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Anthropic stream emitted unsupported event type `{event_type}`"
                )));
            }
            None => {
                return Err(StreamTranslationError::Semantic(
                    "Anthropic stream event is missing `type`".to_string(),
                ));
            }
        }
        let parsed = serde_json::from_value::<MessageStreamEvent>(payload)?;
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
            .require_streaming_phase_mut(RequireStreamingPhaseContext {
                source: "Anthropic",
                event: "active content event",
            })
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

    pub(crate) fn unexpected_stream_end_error(
        &self,
        end: SseStreamEnd,
        target_protocol_label: &'static str,
    ) -> StreamTranslationError {
        let message = match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                format!("Anthropic stream reached {end} before message_start")
            }
            InboundStreamLifecyclePhase::Terminal => {
                format!(
                    "Anthropic stream reached {end} after terminal message_delta but before message_stop"
                )
            }
            InboundStreamLifecyclePhase::Streaming => {
                let phase = self
                    .inner
                    .streaming_phase()
                    .expect("streaming phase exists");
                if phase.emitted_any() {
                    format!("Anthropic stream reached {end} before terminal message_delta")
                } else {
                    format!(
                        "Anthropic stream completed without {target_protocol_label}-representable content, thinking, refusal, or tool_use blocks"
                    )
                }
            }
            InboundStreamLifecyclePhase::Stopped => String::new(),
        };
        StreamTranslationError::Semantic(message)
    }
}
