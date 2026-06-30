//! Inbound-side streaming lifecycle shared by every translator rooted at
//! `anthropic_messages`.
//!
//! This module owns the parts of Anthropic Messages stream translation that are
//! the same regardless of the target protocol:
//!
//! - the four-phase state machine (`WaitingForMessageStart` → `Streaming` →
//!   `ReceivedTerminalDelta` → `Stopped`),
//! - the protocol-ordering rules enforced on every inbound event,
//! - the per-pair state getter helpers (`streaming_state`,
//!   `take_streaming_state`, `take_terminal_state`),
//! - the inbound event-type allowlist,
//! - the "stream ended unexpectedly" error template.
//!
//! Target-protocol-specific behavior (block registration, output item id
//! allocation, refusal text handling, ...) stays on the per-pair
//! `StreamingState`, which implements [`AnthropicStreamState`] so the lifecycle
//! can query representable-output progress without knowing the concrete state
//! type.
//!
//! This module deliberately depends on `protocol::anthropic::messages` (the
//! inbound wire shape) and on the protocol-agnostic `translation::streaming`
//! helpers, but the reverse dependencies do not hold: the protocol-agnostic
//! streaming layer never imports Anthropic-specific types.

use serde_json::Value;

use crate::protocol::anthropic::messages::MessageStreamEvent;
use crate::translation::streaming::{
    SseStreamEnd, StreamTranslationError, StreamTranslationResult,
};

/// Four-phase lifecycle for an Anthropic Messages inbound stream.
///
/// Anthropic Messages streams follow a fixed protocol independent of the target
/// protocol:
///
/// 1. `message_start` opens the assistant message envelope.
/// 2. `content_block_start` / `content_block_delta` / `content_block_stop` and
///    `ping` flow during the streaming phase.
/// 3. a single terminal `message_delta` carries the stop reason.
/// 4. `message_stop` ends the stream.
///
/// Every translator out of Anthropic Messages enforces the same ordering
/// invariants on top of this shape, so the rules and the state getters live
/// here once instead of being copy-pasted per target protocol.
///
/// `Default` derives to `WaitingForMessageStart`, which carries no per-pair
/// state, so `S` itself does not need to implement `Default`. This lets
/// callers `std::mem::take` the lifecycle without constraining the per-pair
/// state type.
#[derive(Debug, Default)]
pub(crate) enum AnthropicInboundLifecycle<S> {
    #[default]
    WaitingForMessageStart,
    Streaming(S),
    ReceivedTerminalDelta(S),
    Stopped,
}

/// Per-pair streaming state hook for [`AnthropicInboundLifecycle`].
///
/// The lifecycle state machine only needs to know two target-protocol-specific
/// things: whether the stream has produced any target-representable output
/// (used to tailor the "stream ended unexpectedly" error message), and what the
/// target protocol is called (used in that same error message). Everything
/// else — block registration, item id allocation, refusal text handling —
/// stays on the concrete state struct owned by the pair.
pub(crate) trait AnthropicStreamState {
    /// Whether the stream has emitted any content the target protocol can
    /// express so far.
    fn emitted_any(&self) -> bool;

    /// Human-readable label for the target protocol (e.g. `"Chat"`,
    /// `"Responses"`), substituted into the "stream completed without
    /// X-representable content" error message.
    fn target_protocol_label() -> &'static str;
}

impl<S> AnthropicInboundLifecycle<S> {
    /// Enforce Anthropic Messages stream ordering rules for an incoming event.
    ///
    /// `Ping` events are always allowed (they carry no semantic payload). All
    /// other events are gated by the current lifecycle phase:
    ///
    /// - `WaitingForMessageStart` only permits `MessageStart`.
    /// - `Streaming` rejects `MessageStop` (it must follow a terminal delta).
    /// - `ReceivedTerminalDelta` only permits `MessageStop`.
    /// - `Stopped` rejects everything.
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

    /// Borrow the active streaming state, or error if the lifecycle is not in
    /// the `Streaming` phase.
    pub(crate) fn streaming_state(&self) -> StreamTranslationResult<&S> {
        match self {
            Self::Streaming(state) => Ok(state),
            _ => Err(StreamTranslationError::Semantic(
                "Anthropic stream active content event occurred outside streaming state"
                    .to_string(),
            )),
        }
    }

    /// Mutably borrow the active streaming state, or error if the lifecycle is
    /// not in the `Streaming` phase.
    pub(crate) fn streaming_state_mut(&mut self) -> StreamTranslationResult<&mut S> {
        match self {
            Self::Streaming(state) => Ok(state),
            _ => Err(StreamTranslationError::Semantic(
                "Anthropic stream active content event occurred outside streaming state"
                    .to_string(),
            )),
        }
    }

    /// Move the streaming state out of the `Streaming` phase, leaving the
    /// lifecycle at its `Default` (`WaitingForMessageStart`). The caller is
    /// expected to immediately reinstall a new phase (typically
    /// `ReceivedTerminalDelta(state)`).
    pub(crate) fn take_streaming_state(&mut self) -> StreamTranslationResult<S> {
        match std::mem::take(self) {
            Self::Streaming(state) => Ok(state),
            other => {
                *self = other;
                Err(StreamTranslationError::Semantic(
                    "Anthropic stream terminal event occurred outside streaming state".to_string(),
                ))
            }
        }
    }

    /// Move the state out of the `ReceivedTerminalDelta` phase when
    /// `message_stop` arrives, leaving the lifecycle at `Stopped` (via
    /// `Default` + caller reinstall). The caller is expected to set
    /// `Stopped` immediately afterwards.
    pub(crate) fn take_terminal_state(&mut self) -> StreamTranslationResult<S> {
        match std::mem::take(self) {
            Self::ReceivedTerminalDelta(state) => Ok(state),
            other => {
                *self = other;
                Err(StreamTranslationError::Semantic(
                    "Anthropic stream message_stop occurred before terminal message_delta"
                        .to_string(),
                ))
            }
        }
    }

    /// Whether the lifecycle has reached the terminal `Stopped` phase.
    pub(crate) fn is_stopped(&self) -> bool {
        matches!(self, Self::Stopped)
    }

    /// Build the error returned when the byte stream ends (EOF or `[DONE]`)
    /// without the lifecycle reaching `Stopped`.
    ///
    /// Callers should check [`is_stopped`](Self::is_stopped) first and return
    /// an empty vec; reaching this method implies an unexpected end.
    pub(crate) fn unexpected_stream_end_error(&self, end: SseStreamEnd) -> StreamTranslationError
    where
        S: AnthropicStreamState,
    {
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
            Self::Streaming(state) if state.emitted_any() => {
                format!("Anthropic stream reached {end_label} before terminal message_delta")
            }
            Self::Streaming(_) => format!(
                "Anthropic stream completed without {}-representable content, thinking, refusal, or tool_use blocks",
                S::target_protocol_label()
            ),
            Self::Stopped => String::new(),
        };
        StreamTranslationError::Semantic(message)
    }
}

/// Validate that an inbound SSE payload carries an Anthropic Messages stream
/// event type that proxai knows how to translate.
///
/// The allowlist is the same for every target protocol because it describes the
/// source protocol's wire shape, not the target's.
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
