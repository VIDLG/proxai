//! Protocol-neutral structured stream values and inbound lifecycle primitives.
//!
//! This module has no dependency on protocol-pair implementations or the
//! `Translator` façade. Pair state machines depend on these types; `Translator`
//! composes pair state with route and observation capabilities.

use std::pin::Pin;

use delegate::delegate;
use futures_util::Stream;
use getset::{Getters, MutGetters};
use serde::Serialize;
use serde_json::Value;
use strum::{Display, EnumDiscriminants};

use crate::error::JsonPayloadError;

const DEFAULT_EVENT_TYPE: &str = "message";
const DONE_SENTINEL_DATA: &str = "[DONE]";

pub type StreamTranslationResult<T> = Result<T, StreamTranslationError>;

#[derive(Debug, thiserror::Error)]
pub enum StreamTranslationError {
    #[error("stream payload conversion failed: {0}")]
    Translation(#[from] super::error::TranslationError),

    #[error(transparent)]
    JsonPayload(#[from] JsonPayloadError),

    #[error("stream JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("stream semantic conversion failed: {0}")]
    Semantic(String),
}

impl StreamTranslationError {
    pub fn as_json_payload_error(&self) -> Option<&JsonPayloadError> {
        match self {
            Self::Translation(error) => error.as_json_payload_error(),
            Self::JsonPayload(error) => Some(error),
            Self::Json(_) | Self::Semantic(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
pub enum StreamEnd {
    #[strum(serialize = "[DONE]")]
    Done,
    #[strum(serialize = "EOF")]
    Eof,
}

/// A parsed SSE event with its JSON payload, serving as the I/O unit for
/// stream translators.
///
/// Translators receive and produce `StreamEvent` values; they never touch
/// raw `Bytes` or `SseEvent`. The carrier layer
/// (`translate_sse_stream`) is responsible for parsing
/// `SseEvent` → `StreamEvent` on input and
/// encoding `StreamEvent` → `Bytes` on output.
#[derive(Debug, Clone)]
pub struct StreamEvent {
    pub event_type: String,
    pub data: Value,
}

impl StreamEvent {
    pub fn new(event_type: impl Into<String>, data: Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }

    pub fn json(
        event_type: impl Into<String>,
        payload: impl Serialize,
    ) -> StreamTranslationResult<Self> {
        Ok(Self::new(event_type, serde_json::to_value(payload)?))
    }

    pub fn message(payload: impl Serialize) -> StreamTranslationResult<Self> {
        Self::json(DEFAULT_EVENT_TYPE, payload)
    }

    /// Chat Completions `[DONE]` sentinel, expressed as a `StreamEvent`.
    ///
    /// The carrier layer recognizes this combination (`event_type` = `message`
    /// and `data` = string `"[DONE]"`) and emits the raw `data: [DONE]` frame
    /// instead of JSON-serializing the payload.
    pub fn done() -> Self {
        Self {
            event_type: DEFAULT_EVENT_TYPE.to_string(),
            data: Value::String(DONE_SENTINEL_DATA.to_string()),
        }
    }

    /// Returns whether this event represents the Chat Completions `[DONE]` sentinel.
    pub fn is_done_sentinel(&self) -> bool {
        self.event_type == DEFAULT_EVENT_TYPE
            && self.data == Value::String(DONE_SENTINEL_DATA.to_string())
    }
}

pub(crate) fn typed_stream_event<E>(event: E) -> StreamTranslationResult<StreamEvent>
where
    E: AsRef<str> + Serialize,
{
    let event_type = event.as_ref().to_string();
    StreamEvent::json(event_type, event)
}

pub(crate) fn typed_stream_events<E>(
    events: impl IntoIterator<Item = E>,
) -> StreamTranslationResult<Vec<StreamEvent>>
where
    E: AsRef<str> + Serialize,
{
    events.into_iter().map(typed_stream_event).collect()
}

#[derive(Debug, Clone)]
pub enum StreamTranslationInput {
    Event(StreamEvent),
    End(StreamEnd),
}

pub type StreamEventStream =
    Pin<Box<dyn Stream<Item = StreamTranslationResult<StreamEvent>> + Send + 'static>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamIdentity {
    id: String,
    model: String,
}

impl StreamIdentity {
    pub(crate) fn new(id: String, model: String) -> Self {
        Self { id, model }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Getters, MutGetters)]
pub(crate) struct StreamingPhase<S> {
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    state: S,
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    output: EmittedContentTracker,
}

impl<S> StreamingPhase<S> {
    pub(crate) fn new(state: S) -> Self {
        Self {
            state,
            output: EmittedContentTracker::default(),
        }
    }

    pub(crate) fn into_state(self) -> S {
        self.state
    }

    delegate! {
        to self.output {
            pub(crate) fn mark_text(&mut self);
            pub(crate) fn mark_refusal(&mut self);
            pub(crate) fn mark_tool_use(&mut self);
            pub(crate) fn mark_reasoning(&mut self);
            pub(crate) fn emitted_text(&self) -> bool;
            pub(crate) fn emitted_any(&self) -> bool;
        }
    }
}

/// Tracks which kinds of representable content the stream has actually
/// emitted into the target protocol so far.
///
/// `mark_*` is called at the moment a block's representable payload is first
/// guaranteed to be non-empty (for example when `content_block_start`
/// arrives with mandatory `id` + `name`, or when the first non-empty text
/// delta is appended). The flags then feed two decisions:
///
/// - `emitted_any()` rejects empty streams when the terminal event arrives
///   ("stream completed without representable content"), and also tailors
///   source-lifecycle errors when the carrier ends too early.
/// - `emitted_text()` lets Chat streaming decide whether a terminal refusal
///   can still be emitted (a refusal cannot retract text that was already
///   sent as content).
///
/// "Content" here means target-protocol representable output. A block that
/// the target protocol cannot express (e.g. redacted thinking when the
/// target is Chat Completions) is simply never marked, so an otherwise empty
/// stream still surfaces as empty.
#[derive(Debug, Default)]
pub(crate) struct EmittedContentTracker {
    emitted_text: bool,
    emitted_refusal: bool,
    emitted_tool_use: bool,
    emitted_reasoning: bool,
}

impl EmittedContentTracker {
    pub(crate) fn mark_text(&mut self) {
        self.emitted_text = true;
    }

    pub(crate) fn mark_refusal(&mut self) {
        self.emitted_refusal = true;
    }

    pub(crate) fn mark_tool_use(&mut self) {
        self.emitted_tool_use = true;
    }

    pub(crate) fn mark_reasoning(&mut self) {
        self.emitted_reasoning = true;
    }

    pub(crate) fn emitted_text(&self) -> bool {
        self.emitted_text
    }

    pub(crate) fn emitted_any(&self) -> bool {
        self.emitted_text || self.emitted_refusal || self.emitted_tool_use || self.emitted_reasoning
    }
}

/// Protocol-neutral inbound stream lifecycle carrier.
///
/// This type owns the mechanical four-phase shape shared by source protocols
/// plus the stream envelope identity once the source stream has started. The
/// identity is stored outside the phase enum because it remains stable across
/// `Streaming`, `Terminal`, and `Stopped`.
#[derive(Debug)]
pub(crate) struct InboundStreamLifecycle<S, T> {
    identity: Option<StreamIdentity>,
    phase: InboundStreamPhase<S, T>,
}

#[derive(Debug, Default, EnumDiscriminants)]
#[strum_discriminants(
    name(InboundStreamLifecyclePhase),
    vis(pub(crate)),
    derive(Display),
    strum(serialize_all = "snake_case")
)]
enum InboundStreamPhase<S, T> {
    #[default]
    Waiting,
    Streaming(StreamingPhase<S>),
    Terminal(T),
    Stopped,
}

impl<S, T> Default for InboundStreamLifecycle<S, T> {
    fn default() -> Self {
        Self {
            identity: None,
            phase: InboundStreamPhase::Waiting,
        }
    }
}

impl<S, T> InboundStreamLifecycle<S, T> {
    pub(crate) fn begin_streaming(&mut self, identity: StreamIdentity, state: S) {
        self.identity = Some(identity);
        self.phase = InboundStreamPhase::Streaming(StreamingPhase::new(state));
    }

    pub(crate) fn receive_terminal(&mut self, terminal: T) {
        self.phase = InboundStreamPhase::Terminal(terminal);
    }

    pub(crate) fn stop(&mut self) {
        self.phase = InboundStreamPhase::Stopped;
    }

    pub(crate) fn require_identity(
        &self,
        error: impl FnOnce() -> StreamTranslationError,
    ) -> StreamTranslationResult<&StreamIdentity> {
        self.identity.as_ref().ok_or_else(error)
    }

    pub(crate) fn is_waiting(&self) -> bool {
        matches!(self.phase, InboundStreamPhase::Waiting)
    }

    pub(crate) fn is_stopped(&self) -> bool {
        matches!(self.phase, InboundStreamPhase::Stopped)
    }

    pub(crate) fn phase_kind(&self) -> InboundStreamLifecyclePhase {
        (&self.phase).into()
    }

    pub(crate) fn streaming_phase(&self) -> Option<&StreamingPhase<S>> {
        match &self.phase {
            InboundStreamPhase::Streaming(phase) => Some(phase),
            _ => None,
        }
    }

    pub(crate) fn require_streaming_phase_mut(
        &mut self,
        source: &'static str,
        event: &'static str,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        let phase_kind = self.phase_kind();
        match &mut self.phase {
            InboundStreamPhase::Streaming(phase) => Ok(phase),
            _ => Err(StreamTranslationError::Semantic(format!(
                "{source} stream emitted {event} while lifecycle was {phase_kind}; expected streaming"
            ))),
        }
    }

    pub(crate) fn terminal(&self) -> Option<&T> {
        match &self.phase {
            InboundStreamPhase::Terminal(terminal) => Some(terminal),
            _ => None,
        }
    }

    pub(crate) fn terminal_mut(&mut self) -> Option<&mut T> {
        match &mut self.phase {
            InboundStreamPhase::Terminal(terminal) => Some(terminal),
            _ => None,
        }
    }

    pub(crate) fn take_streaming_phase(
        &mut self,
        error: impl FnOnce() -> StreamTranslationError,
    ) -> StreamTranslationResult<StreamingPhase<S>> {
        match std::mem::take(&mut self.phase) {
            InboundStreamPhase::Streaming(phase) => Ok(phase),
            other => {
                self.phase = other;
                Err(error())
            }
        }
    }

    pub(crate) fn take_terminal(
        &mut self,
        error: impl FnOnce() -> StreamTranslationError,
    ) -> StreamTranslationResult<T> {
        match std::mem::take(&mut self.phase) {
            InboundStreamPhase::Terminal(terminal) => Ok(terminal),
            other => {
                self.phase = other;
                Err(error())
            }
        }
    }
}
