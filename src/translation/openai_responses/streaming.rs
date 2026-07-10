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

use std::collections::{BTreeMap, BTreeSet};

use delegate::delegate;
use serde_json::Value;
use strum::Display;

use crate::protocol::openai::responses::{OutputItem, ResponseStreamEvent};
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
    completed_output_items: BTreeSet<u32>,
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

#[derive(Debug)]
struct ObservedOutputItem {
    kind: ResponsesOutputKind,
    item_id: Option<String>,
}

impl<S> ResponsesInboundLifecycle<S> {
    delegate! {
        to self.inner {
            pub(crate) fn is_stopped(&self) -> bool;
            pub(crate) fn stop(&mut self);
        }
    }

    /// Parse a Responses SSE event payload into the typed source event model.
    ///
    /// Responses exposes many semantic event kinds. This parser only decodes the
    /// typed source event; source event ordering is enforced by the inbound
    /// lifecycle and each pair translator's target-side state.
    pub(crate) fn parse_stream_event(
        &self,
        payload: Value,
    ) -> StreamTranslationResult<ResponseStreamEvent> {
        serde_json::from_value::<ResponseStreamEvent>(payload).map_err(Into::into)
    }

    /// Observe a Responses lifecycle snapshot (`response.created` / `in_progress`).
    ///
    /// The first snapshot initializes stream identity and pair state. Later
    /// snapshots are valid only while streaming and must refer to the same
    /// response identity; they do not reset pair state.
    pub(crate) fn observe_response_stream(
        &mut self,
        identity: StreamIdentity,
        state: impl FnOnce() -> S,
    ) -> StreamTranslationResult<bool> {
        if self.inner.is_waiting() {
            self.inner.begin_streaming(identity, state());
            return Ok(true);
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
        Ok(false)
    }

    pub(crate) fn validate_stream_event(
        &mut self,
        event: &ResponseStreamEvent,
    ) -> StreamTranslationResult<()> {
        let event_type = event.as_ref();
        match event {
            ResponseStreamEvent::ResponseOutputItemAdded(event) => {
                self.register_output_item(event.output_index, &event.item, event_type)
            }
            ResponseStreamEvent::ResponseOutputItemDone(event) => {
                self.complete_output_item(event.output_index, &event.item, event_type)
            }
            ResponseStreamEvent::ResponseContentPartAdded(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                output_content_kind(&event.part),
                event_type,
            ),
            ResponseStreamEvent::ResponseContentPartDone(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                output_content_kind(&event.part),
                event_type,
            ),
            ResponseStreamEvent::ResponseOutputTextDelta(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseOutputTextDone(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseRefusalDelta(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseRefusalDone(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Message,
                event_type,
            ),
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Reasoning,
                    event_type,
                ),
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Reasoning,
                event_type,
            ),
            ResponseStreamEvent::ResponseReasoningTextDone(event) => self.require_output_item(
                event.output_index,
                &event.item_id,
                ResponsesOutputKind::Reasoning,
                event_type,
            ),
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::FunctionCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::FunctionCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseCustomToolCallInputDelta(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::CustomToolCall,
                    event_type,
                ),
            ResponseStreamEvent::ResponseCustomToolCallInputDone(event) => self
                .require_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::CustomToolCall,
                    event_type,
                ),
            _ => Ok(()),
        }
    }

    pub(crate) fn register_output_item(
        &mut self,
        output_index: u32,
        item: &OutputItem,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if self.output_items.contains_key(&output_index)
            || self.completed_output_items.contains(&output_index)
        {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }
        self.output_items.insert(
            output_index,
            ObservedOutputItem {
                kind: output_kind(item),
                item_id: output_item_id(item).map(ToOwned::to_owned),
            },
        );
        Ok(())
    }

    pub(crate) fn require_output_item(
        &mut self,
        output_index: u32,
        item_id: &str,
        expected_kind: ResponsesOutputKind,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if self.completed_output_items.contains(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} after response.output_item.done"
            )));
        }
        let observed = self.output_items.get_mut(&output_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            ))
        })?;
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

    pub(crate) fn complete_output_item(
        &mut self,
        output_index: u32,
        item: &OutputItem,
        event: &str,
    ) -> StreamTranslationResult<()> {
        if self.completed_output_items.contains(&output_index) {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted duplicate {event} for output_index {output_index}"
            )));
        }
        let observed = self.output_items.get_mut(&output_index).ok_or_else(|| {
            StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} for output_index {output_index} before response.output_item.added"
            ))
        })?;
        let completed_kind = output_kind(item);
        if observed.kind != completed_kind {
            return Err(StreamTranslationError::Semantic(format!(
                "Responses stream emitted {event} with {completed_kind} item for {} output_index {output_index}",
                observed.kind
            )));
        }
        if let Some(item_id) = output_item_id(item) {
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
        self.completed_output_items.insert(output_index);
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

fn output_content_kind(
    content: &crate::protocol::openai::responses::OutputContent,
) -> ResponsesOutputKind {
    match content {
        crate::protocol::openai::responses::OutputContent::OutputText(_)
        | crate::protocol::openai::responses::OutputContent::Refusal(_) => {
            ResponsesOutputKind::Message
        }
        crate::protocol::openai::responses::OutputContent::ReasoningText(_) => {
            ResponsesOutputKind::Reasoning
        }
    }
}

fn output_kind(item: &OutputItem) -> ResponsesOutputKind {
    match item {
        OutputItem::Message(_) => ResponsesOutputKind::Message,
        OutputItem::Reasoning(_) => ResponsesOutputKind::Reasoning,
        OutputItem::FunctionCall(_) => ResponsesOutputKind::FunctionCall,
        OutputItem::CustomToolCall(_) => ResponsesOutputKind::CustomToolCall,
        _ => ResponsesOutputKind::Other,
    }
}

fn output_item_id(item: &OutputItem) -> Option<&str> {
    match item {
        OutputItem::Message(item) => Some(&item.id),
        OutputItem::Reasoning(item) => item.id.as_deref(),
        OutputItem::FunctionCall(item) => item.id.as_deref(),
        OutputItem::CustomToolCall(item) => Some(&item.id),
        _ => None,
    }
}
