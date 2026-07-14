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
//! envelope: identity initialization, terminal detection, and source-side typed
//! event parsing.

use std::collections::BTreeMap;

use delegate::delegate;
use derive_more::From;
use serde_json::Value;
use strum::Display;

use crate::protocol::openai::responses::{
    OutputContent, OutputItem, Response, ResponseStreamEvent,
};
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
    output_items: BTreeMap<u32, ObservedOutputItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum ResponsesOutputKind {
    Message,
    Reasoning,
    FunctionCall,
    CustomToolCall,
    Other,
}

impl From<&OutputItem> for ResponsesOutputKind {
    fn from(item: &OutputItem) -> Self {
        match item {
            OutputItem::Message(_) => Self::Message,
            OutputItem::Reasoning(_) => Self::Reasoning,
            OutputItem::FunctionCall(_) => Self::FunctionCall,
            OutputItem::CustomToolCall(_) => Self::CustomToolCall,
            _ => Self::Other,
        }
    }
}

impl From<&OutputContent> for ResponsesOutputKind {
    fn from(content: &OutputContent) -> Self {
        match content {
            OutputContent::OutputText(_) | OutputContent::Refusal(_) => Self::Message,
            OutputContent::ReasoningText(_) => Self::Reasoning,
        }
    }
}

#[derive(Debug)]
struct ObservedOutputItem {
    kind: ResponsesOutputKind,
    item_id: Option<String>,
    completed: bool,
}

pub(crate) fn response_failure_error(response: &Response) -> StreamTranslationError {
    let detail = response
        .error
        .as_non_null()
        .map(|error| format!("{}: {}", error.code, error.message))
        .unwrap_or_else(|| "upstream response failed without error details".to_string());
    StreamTranslationError::Semantic(format!("Responses stream failed: {detail}"))
}

/// Source-side identity for one incremental Responses output segment.
///
/// This is a wire-coordinate key only: it deliberately contains neither an
/// Anthropic `content_block.index` nor a Chat tool-call index. Target pairs may
/// use it to track projection state or reconcile authoritative `*.done`
/// snapshots without duplicating their source coordinate model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ResponsesOutputSegmentKey {
    Text {
        output_index: u32,
        content_index: u32,
    },
    Refusal {
        output_index: u32,
        content_index: u32,
    },
    ReasoningText {
        output_index: u32,
        content_index: u32,
    },
    ReasoningSummary {
        output_index: u32,
        summary_index: u32,
    },
    FunctionArguments {
        output_index: u32,
    },
    CustomToolInput {
        output_index: u32,
    },
}

impl ResponsesOutputSegmentKey {
    pub(crate) fn output_index(self) -> u32 {
        match self {
            Self::Text { output_index, .. }
            | Self::Refusal { output_index, .. }
            | Self::ReasoningText { output_index, .. }
            | Self::ReasoningSummary { output_index, .. }
            | Self::FunctionArguments { output_index }
            | Self::CustomToolInput { output_index } => output_index,
        }
    }

    pub(crate) fn is_reasoning(self) -> bool {
        matches!(
            self,
            Self::ReasoningText { .. } | Self::ReasoningSummary { .. }
        )
    }

    pub(crate) fn is_tool_input(self) -> bool {
        matches!(
            self,
            Self::FunctionArguments { .. } | Self::CustomToolInput { .. }
        )
    }

    pub(crate) fn context(self) -> String {
        match self {
            Self::Text {
                output_index,
                content_index,
            } => format!("text output_index {output_index} content_index {content_index}"),
            Self::Refusal {
                output_index,
                content_index,
            } => format!("refusal output_index {output_index} content_index {content_index}"),
            Self::ReasoningText {
                output_index,
                content_index,
            } => format!("reasoning output_index {output_index} content_index {content_index}"),
            Self::ReasoningSummary {
                output_index,
                summary_index,
            } => format!(
                "reasoning summary output_index {output_index} summary_index {summary_index}"
            ),
            Self::FunctionArguments { output_index } => {
                format!("function arguments output_index {output_index}")
            }
            Self::CustomToolInput { output_index } => {
                format!("custom tool input output_index {output_index}")
            }
        }
    }
}

#[derive(Debug, Default, From)]
pub(crate) struct ForwardedContent(String);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForwardedContentDivergence;

impl ForwardedContent {
    pub(crate) fn append(&mut self, content: &str) {
        self.0.push_str(content);
    }

    pub(crate) fn reconcile_snapshot(
        &mut self,
        final_content: &str,
    ) -> Result<Option<String>, ForwardedContentDivergence> {
        if !final_content.starts_with(&self.0) {
            return Err(ForwardedContentDivergence);
        }
        let suffix = final_content[self.0.len()..].to_string();
        if suffix.is_empty() {
            return Ok(None);
        }
        self.append(&suffix);
        Ok(Some(suffix))
    }
}

impl<S> ResponsesInboundLifecycle<S> {
    delegate! {
        to self.inner {
            pub(crate) fn is_stopped(&self) -> bool;
            pub(crate) fn stop(&mut self);
        }
    }

    /// Parse a Responses SSE event payload and apply source-side output-item
    /// lifecycle validation before returning the typed event.
    pub(crate) fn parse_stream_event(
        &mut self,
        payload: Value,
    ) -> StreamTranslationResult<ResponseStreamEvent> {
        let parsed = serde_json::from_value::<ResponseStreamEvent>(payload)?;
        self.apply_output_item_lifecycle(&parsed)?;
        Ok(parsed)
    }

    /// Ensure a Responses stream is initialized for a lifecycle snapshot
    /// (`response.created` / `in_progress`).
    ///
    /// The first snapshot initializes stream identity and pair state. Later
    /// snapshots are valid only while streaming and must refer to the same
    /// response identity; they do not reset pair state.
    pub(crate) fn ensure_response_stream(
        &mut self,
        identity: StreamIdentity,
        state: S,
    ) -> StreamTranslationResult<()> {
        if self.inner.is_waiting() {
            self.inner.begin_streaming(identity, state);
            return Ok(());
        }

        let current = self.inner.require_identity(|| {
            StreamTranslationError::Semantic(
                "Responses stream lifecycle snapshot arrived before identity initialization"
                    .to_string(),
            )
        })?;
        if current != &identity {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream identity changed from {}/{} to {}/{}",
                current.id(),
                current.model(),
                identity.id(),
                identity.model()
            )));
        }
        if !matches!(
            self.inner.phase_kind(),
            InboundStreamLifecyclePhase::Streaming
        ) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted response.created / response.in_progress while lifecycle was {}; expected streaming",
                self.inner.phase_kind()
            )));
        }
        Ok(())
    }

    fn apply_output_item_lifecycle(
        &mut self,
        event: &ResponseStreamEvent,
    ) -> StreamTranslationResult<()> {
        let event_type = event.as_ref();
        match event {
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                self.register_output_item(event.output_index, &event.item, event_type)
            }
            ResponseStreamEvent::ResponseContentPartAdded(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::from(&event.part),
                    event_type,
                ),
            ResponseStreamEvent::ResponseOutputTextDelta(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Message,
                    event_type,
                ),
            ResponseStreamEvent::ResponseOutputTextDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Message,
                    event_type,
                ),
            ResponseStreamEvent::ResponseRefusalDelta(event) => self.observe_required_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseRefusalDone(event) => self.observe_required_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningTextDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseContentPartDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::from(&event.part),
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::FunctionCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::FunctionCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseCustomToolCallInputDelta(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::CustomToolCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseCustomToolCallInputDone(event) => self
                .observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::CustomToolCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseOutputItemDone(event) => {
                self.complete_output_item(event.output_index, &event.item, event_type)
            }
            _ => Ok(()),
        }
    }

    fn require_streaming_output_item_event(&self, event: &str) -> StreamTranslationResult<()> {
        if matches!(
            self.inner.phase_kind(),
            InboundStreamLifecyclePhase::Streaming
        ) {
            return Ok(());
        }

        Err(StreamTranslationError::Semantic(format!(
            "Responses stream emitted {event} while lifecycle was {}; expected streaming",
            self.inner.phase_kind()
        )))
    }

    fn register_output_item(
        &mut self,
        output_index: u32,
        item: &OutputItem,
        event: &str,
    ) -> StreamTranslationResult<()> {
        self.require_streaming_output_item_event(event)?;
        match item {
            OutputItem::Message(message) if !message.content.is_empty() => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream emitted {event} with non-empty message content; content must be emitted through response.content_part.* events"
                )));
            }
            OutputItem::Reasoning(reasoning)
                if !reasoning.summary.is_empty()
                    || reasoning
                        .content
                        .as_ref()
                        .is_some_and(|content| !content.is_empty()) =>
            {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream emitted {event} with reasoning content or summary; reasoning must be emitted through response.reasoning_* events"
                )));
            }
            _ => {}
        }
        if self.output_items.contains_key(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }
        self.output_items.insert(
            output_index,
            ObservedOutputItem {
                kind: ResponsesOutputKind::from(item),
                item_id: item.id().map(ToOwned::to_owned),
                completed: false,
            },
        );
        Ok(())
    }

    fn observe_required_output_item(
        &mut self,
        output_index: u32,
        item_id: &str,
        expected_kind: ResponsesOutputKind,
        event: &str,
    ) -> StreamTranslationResult<()> {
        self.require_streaming_output_item_event(event)?;
        let observed = self.output_items.get_mut(&output_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            ))
        })?;
        if observed.completed {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} after response.output_item.done"
            )));
        }
        if observed.kind != expected_kind {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for {} output_index {output_index}; expected {expected_kind}",
                observed.kind
            )));
        }
        match observed.item_id.as_deref() {
            Some(expected_item_id) if expected_item_id != item_id => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream emitted {event} with item_id {item_id} for output_index {output_index}; expected item_id {expected_item_id}"
                )));
            }
            Some(_) => {}
            None => observed.item_id = Some(item_id.to_string()),
        }
        Ok(())
    }

    fn complete_output_item(
        &mut self,
        output_index: u32,
        item: &OutputItem,
        event: &str,
    ) -> StreamTranslationResult<()> {
        self.require_streaming_output_item_event(event)?;
        let observed = self.output_items.get_mut(&output_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            ))
        })?;
        if observed.completed {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }
        let completed_kind = ResponsesOutputKind::from(item);
        if observed.kind != completed_kind {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} with {completed_kind} item for {} output_index {output_index}",
                observed.kind
            )));
        }
        if let Some(item_id) = item.id() {
            match observed.item_id.as_deref() {
                Some(expected_item_id) if expected_item_id != item_id => {
                    return Err(StreamTranslationError::Semantic(format!(
                        "Responses stream emitted {event} with item_id {item_id} for output_index {output_index}; expected item_id {expected_item_id}"
                    )));
                }
                Some(_) => {}
                None => observed.item_id = Some(item_id.to_string()),
            }
        }
        observed.completed = true;
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

    /// Validate a carrier stream ending after a Responses stream has already
    /// emitted its semantic terminal event.
    pub(crate) fn finish_stream(&self, end: SseStreamEnd) -> StreamTranslationResult<()> {
        if self.is_stopped() {
            return Ok(());
        }
        Err(self.unexpected_stream_end_error(end))
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

#[cfg(test)]
#[path = "streaming_tests.rs"]
mod tests;
