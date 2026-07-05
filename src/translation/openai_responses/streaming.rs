//! Inbound-side streaming lifecycle shared by translators rooted at
//! `openai_responses`.
//!
//! Responses SSE streams carry their own lifecycle events
//! (`response.created` / `response.completed` / etc.), so this wrapper mirrors
//! the four-phase `InboundStreamLifecycle` shape used by Anthropic and Chat
//! Completions inbound protocols while exposing the response-level identity
//! (id + model) and usage snapshot that every `responses -> *` translator
//! needs.
//!
//! Target-specific state (block tracking, sequence numbers, output assembly)
//! stays inside each pair translator; this module owns only the source-protocol
//! envelope: identity initialization, terminal detection, and source-side
//! semantic event allowlisting.

use delegate::delegate;
use serde_json::Value;

use crate::protocol::openai::responses::ResponseStreamEvent;
use crate::translation::streaming::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, RequireStreamingPhaseContext,
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult,
};

/// Inbound lifecycle wrapper for translators rooted at `openai_responses`.
///
/// `S` is the pair-private streaming state (e.g. open block tracking). The
/// lifecycle has no separate terminal payload — Responses terminal events
/// (`response.completed` / `incomplete` / `failed`) carry no pair-private
/// data beyond what the streaming state already accumulated, so the terminal
/// phase reuses `S`.
#[derive(Debug, Default)]
pub(crate) struct ResponsesInboundLifecycle<S> {
    inner: InboundStreamLifecycle<S, S>,
}

impl<S> ResponsesInboundLifecycle<S> {
    delegate! {
        to self.inner {
            pub(crate) fn is_stopped(&self) -> bool;
        }
    }

    /// Parse and allowlist a Responses SSE event payload.
    ///
    /// Unknown event types are accepted but logged at the caller; semantic
    /// ordering is enforced loosely because Responses does not require a strict
    /// `created -> content -> completed` order beyond what each translator
    /// already checks.
    pub(crate) fn parse_stream_event(
        &self,
        payload: Value,
    ) -> StreamTranslationResult<ResponseStreamEvent> {
        serde_json::from_value::<ResponseStreamEvent>(payload).map_err(Into::into)
    }

    /// Begin a Responses stream when `response.created` / `in_progress` arrives.
    pub(crate) fn begin_response_stream(
        &mut self,
        identity: StreamIdentity,
        state: S,
    ) -> StreamTranslationResult<()> {
        if !self.inner.is_waiting() {
            return Err(StreamTranslationError::Semantic(
                "Responses stream emitted duplicate response.created / response.in_progress"
                    .to_string(),
            ));
        }
        self.inner.begin_streaming(identity, state);
        Ok(())
    }

    /// Move from streaming to terminal when a `response.completed` /
    /// `incomplete` / `failed` event arrives, carrying the streaming state
    /// forward so the translator can finalize target-side output.
    pub(crate) fn receive_terminal_event(&mut self) -> StreamTranslationResult<()> {
        let phase = self.inner.take_streaming_phase(|| {
            StreamTranslationError::Semantic(
                "Responses terminal event occurred before response.created".to_string(),
            )
        })?;
        self.inner.receive_terminal(phase.into_state());
        Ok(())
    }

    /// Mark the carrier stream as fully stopped after a terminal event.
    pub(crate) fn stop(&mut self) {
        self.inner.stop();
    }

    pub(crate) fn stream_identity(&self) -> StreamTranslationResult<&StreamIdentity> {
        self.inner.require_identity(|| {
            StreamTranslationError::Semantic(
                "Responses stream identity is not initialized before response.created".to_string(),
            )
        })
    }

    pub(crate) fn streaming_state_mut(&mut self) -> StreamTranslationResult<&mut S> {
        let phase = self
            .inner
            .require_streaming_phase_mut(RequireStreamingPhaseContext {
                source: "Responses",
                event: "active content event",
            })?;
        Ok(phase.state_mut())
    }

    /// Take the streaming state when finalizing after a terminal event.
    pub(crate) fn take_terminal_state(&mut self) -> StreamTranslationResult<S> {
        self.inner.take_terminal(|| {
            StreamTranslationError::Semantic(
                "Responses stream finalized before a terminal response event".to_string(),
            )
        })
    }

    /// Build an error describing why the carrier stream ended unexpectedly.
    pub(crate) fn unexpected_stream_end_error(&self, end: SseStreamEnd) -> StreamTranslationError {
        let message = match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                format!("Responses stream reached {end} before response.created")
            }
            InboundStreamLifecyclePhase::Streaming => {
                format!("Responses stream reached {end} before a terminal response event")
            }
            InboundStreamLifecyclePhase::Terminal => {
                format!(
                    "Responses stream reached {end} after terminal event but before carrier close"
                )
            }
            InboundStreamLifecyclePhase::Stopped => String::new(),
        };
        StreamTranslationError::Semantic(message)
    }
}
