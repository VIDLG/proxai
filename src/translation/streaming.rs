use async_stream::try_stream;
use axum::body::Bytes;
use delegate::delegate;
use futures_util::StreamExt;
use getset::{Getters, MutGetters};

use serde::Serialize;
use serde_json::Value;
use strum::Display;

use crate::error::ErrorResponseFields;
use crate::http_support::{ByteStream, into_byte_stream};

pub(crate) use crate::sse::encode_sse_json;
use crate::sse::{DONE_SENTINEL_DATA, SseError, SseEvent, SseEventScanner};

pub(crate) type StreamTranslationResult<T> = Result<T, StreamTranslationError>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StreamTranslationError {
    #[error("stream JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("stream semantic conversion failed: {0}")]
    Semantic(String),

    #[error(transparent)]
    Sse(#[from] SseError),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum StreamTranslatorErrorStage {
    Event,
    Finish,
}

impl StreamTranslatorErrorStage {
    fn default_error_prefix(self) -> &'static str {
        match self {
            Self::Event => "stream translation error",
            Self::Finish => "stream translation finish error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SseStreamEnd {
    DoneSentinel,
    Eof,
}

impl std::fmt::Display for SseStreamEnd {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DoneSentinel => formatter.write_str("[DONE]"),
            Self::Eof => formatter.write_str("EOF"),
        }
    }
}

/// A parsed SSE event with its JSON payload, serving as the I/O unit for
/// stream translators.
///
/// Translators receive and produce `StreamEvent` values; they never touch
/// raw `Bytes` or `SseEvent`. The carrier layer (`translate_sse_stream`)
/// is responsible for parsing `SseEvent` → `StreamEvent` on input and
/// encoding `StreamEvent` → `Bytes` on output.
#[derive(Debug, Clone)]
pub(crate) struct StreamEvent {
    pub event_type: String,
    pub data: Value,
}

impl StreamEvent {
    pub(crate) fn new(event_type: impl Into<String>, data: Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
        }
    }

    pub(crate) fn json(
        event_type: impl Into<String>,
        payload: impl Serialize,
    ) -> StreamTranslationResult<Self> {
        Ok(Self::new(event_type, serde_json::to_value(payload)?))
    }

    pub(crate) fn message(payload: impl Serialize) -> StreamTranslationResult<Self> {
        Self::json(SseEvent::DEFAULT_EVENT_TYPE, payload)
    }

    /// Chat Completions `[DONE]` sentinel, expressed as a `StreamEvent`.
    ///
    /// The carrier layer recognizes this combination (`event_type` = `message`
    /// and `data` = string `"[DONE]"`) and emits the raw `data: [DONE]` frame
    /// instead of JSON-serializing the payload.
    pub(crate) fn done() -> Self {
        Self {
            event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
            data: Value::String(DONE_SENTINEL_DATA.to_string()),
        }
    }

    fn is_done_sentinel(&self) -> bool {
        self.event_type == SseEvent::DEFAULT_EVENT_TYPE
            && self.data == Value::String(DONE_SENTINEL_DATA.to_string())
    }

    fn encode(&self) -> serde_json::Result<Bytes> {
        if self.is_done_sentinel() {
            return Ok(crate::sse::done_sentinel_bytes());
        }
        encode_sse_json(&self.event_type, &self.data)
    }
}

pub(crate) trait StreamingEventTranslator: Send + 'static {
    fn translate_event(&mut self, event: StreamEvent) -> StreamTranslationResult<Vec<StreamEvent>>;

    fn finish_stream(&mut self, _end: SseStreamEnd) -> StreamTranslationResult<Vec<StreamEvent>> {
        Ok(Vec::new())
    }
}

pub(crate) fn translate_sse_stream<T>(input: ByteStream, mut translator: T) -> ByteStream
where
    T: StreamingEventTranslator,
{
    let stream = try_stream! {
        futures_util::pin_mut!(input);

        let mut scanner = SseEventScanner::default();

        while let Some(chunk) = input.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    let event = ErrorResponseFields::upstream_response_body_read(format!(
                        "upstream SSE stream error: {error}"
                    ))
                    .encode_sse_event()?;
                    yield event;
                    return;
                }
            };

            for event in scanner.scan(&chunk) {
                if event.is_done_sentinel() {
                    let events = match translator.finish_stream(SseStreamEnd::DoneSentinel) {
                        Ok(events) => events,
                        Err(error) => {
                            let event = ErrorResponseFields::stream_translation(format!(
                                "{}: {error}",
                                StreamTranslatorErrorStage::Finish.default_error_prefix()
                            ))
                            .encode_sse_event()?;
                            yield event;
                            return;
                        }
                    };
                    for translated in events {
                        yield translated.encode()?;
                    }
                    return;
                }

                let data = event.payload_with_type()?;
                let stream_event = StreamEvent {
                    event_type: event.event_type,
                    data,
                };
                let events = match translator.translate_event(stream_event) {
                    Ok(events) => events,
                    Err(error) => {
                        let event = ErrorResponseFields::stream_translation(format!(
                            "{}: {error}",
                            StreamTranslatorErrorStage::Event.default_error_prefix()
                        ))
                        .encode_sse_event()?;
                        yield event;
                        return;
                    }
                };
                for translated in events {
                    yield translated.encode()?;
                }
            }
        }

        let events = match translator.finish_stream(SseStreamEnd::Eof) {
            Ok(events) => events,
            Err(error) => {
                let event = ErrorResponseFields::stream_translation(format!(
                    "{}: {error}",
                    StreamTranslatorErrorStage::Finish.default_error_prefix()
                ))
                .encode_sse_event()?;
                yield event;
                return;
            }
        };
        for translated in events {
            yield translated.encode()?;
        }
    };
    into_byte_stream(stream.map(|chunk: StreamTranslationResult<Bytes>| chunk))
}

#[derive(Debug, Clone)]
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
///   ("stream completed without representable content"), and is also used by
///   `unexpected_stream_end_error` to tailor error messages.
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

#[derive(Debug)]
enum InboundStreamPhase<S, T> {
    Waiting,
    Streaming(StreamingPhase<S>),
    Terminal(T),
    Stopped,
}

impl<S, T> Default for InboundStreamPhase<S, T> {
    fn default() -> Self {
        Self::Waiting
    }
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
        match self.phase {
            InboundStreamPhase::Waiting => InboundStreamLifecyclePhase::Waiting,
            InboundStreamPhase::Streaming(_) => InboundStreamLifecyclePhase::Streaming,
            InboundStreamPhase::Terminal(_) => InboundStreamLifecyclePhase::Terminal,
            InboundStreamPhase::Stopped => InboundStreamLifecyclePhase::Stopped,
        }
    }

    pub(crate) fn streaming_phase(&self) -> Option<&StreamingPhase<S>> {
        match &self.phase {
            InboundStreamPhase::Streaming(phase) => Some(phase),
            _ => None,
        }
    }

    pub(crate) fn require_streaming_phase_mut(
        &mut self,
        context: RequireStreamingPhaseContext,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        let phase_kind = self.phase_kind();
        match &mut self.phase {
            InboundStreamPhase::Streaming(phase) => Ok(phase),
            _ => Err(StreamTranslationError::Semantic(format!(
                "{} stream emitted {} while lifecycle was {}; expected streaming",
                context.source, context.event, phase_kind
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum InboundStreamLifecyclePhase {
    Waiting,
    Streaming,
    Terminal,
    Stopped,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequireStreamingPhaseContext {
    pub(crate) source: &'static str,
    pub(crate) event: &'static str,
}
