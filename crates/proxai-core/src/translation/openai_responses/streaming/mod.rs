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

mod mantle;

use std::collections::BTreeMap;

use delegate::delegate;
use derive_more::From;
use serde_json::Value;
use strum::Display;

use crate::json::deserialize_value;
use crate::protocol::openai::responses::{
    OutputContent, OutputItem, Response, ResponseErrorEvent, ResponseStreamEvent,
};
use crate::translation::TranslationScope;
use crate::translation::openai_responses::stop::{ResponsesStopKind, infer_response_stop_kind};
use crate::translation::stream::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, StreamEnd, StreamIdentity,
    StreamTranslationError, StreamTranslationResult,
};

pub(crate) use mantle::MantleStreamEvent;

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
    mantle_reasoning: MantleReasoningChannel,
    saw_refusal: bool,
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

/// Lifecycle state for Mantle's coordinate-optional reasoning channel.
///
/// Once a channel starts unscoped it remains unscoped; a later event cannot
/// retroactively re-key already forwarded text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum MantleReasoningChannel {
    #[default]
    Inactive,
    Unscoped,
    Scoped(u32),
}

pub(crate) fn response_failure_error(response: &Response) -> StreamTranslationError {
    let detail = response
        .error
        .as_non_null()
        .map(|error| format!("{}: {}", error.code, error.message))
        .unwrap_or_else(|| "upstream response failed without error details".to_string());
    StreamTranslationError::Semantic(format!("Responses stream failed: {detail}"))
}

pub(crate) fn response_stream_error(event: &ResponseErrorEvent) -> StreamTranslationError {
    let code = event
        .code
        .as_non_null()
        .map(String::as_str)
        .map(|code| format!(" ({code})"))
        .unwrap_or_default();
    StreamTranslationError::Semantic(format!("Responses stream error{code}: {}", event.message))
}

/// Parsed Responses ingress event.
///
/// The official OpenAI union stays isolated in `protocol::openai::responses::wire`.
/// Known provider dialects are represented here because they are accepted only
/// while translating an upstream Responses stream, never advertised as official
/// protocol variants or synthesized during identity forwarding.
#[derive(Debug)]
pub(crate) enum ResponsesInboundStreamEvent {
    /// Event defined by the pinned official OpenAI Responses OpenAPI schema.
    Official(Box<ResponseStreamEvent>),
    /// Known event from AWS Bedrock Mantle's Responses-compatible dialect.
    Mantle(MantleStreamEvent),
}

impl ResponsesInboundStreamEvent {
    pub(crate) fn event_type(&self) -> &str {
        match self {
            Self::Official(event) => event.as_ref().as_ref(),
            Self::Mantle(event) => event.as_ref(),
        }
    }
}

/// Source-side identity for one incremental Responses output segment.
///
/// This is a source-segment key only: it deliberately contains neither an
/// Anthropic `content_block.index` nor a Chat tool-call index. Official events
/// retain their wire coordinates; compatibility channels that provide no stable
/// coordinates remain explicitly unscoped rather than receiving invented ones.
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
    MantleReasoning {
        output_index: Option<u32>,
    },
    FunctionArguments {
        output_index: u32,
    },
    CustomToolInput {
        output_index: u32,
    },
}

impl ResponsesOutputSegmentKey {
    pub(crate) fn output_index(self) -> Option<u32> {
        match self {
            Self::Text { output_index, .. }
            | Self::Refusal { output_index, .. }
            | Self::ReasoningText { output_index, .. }
            | Self::ReasoningSummary { output_index, .. }
            | Self::FunctionArguments { output_index }
            | Self::CustomToolInput { output_index } => Some(output_index),
            Self::MantleReasoning { output_index } => output_index,
        }
    }

    pub(crate) fn is_reasoning(self) -> bool {
        matches!(
            self,
            Self::ReasoningText { .. }
                | Self::ReasoningSummary { .. }
                | Self::MantleReasoning { .. }
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
            Self::MantleReasoning {
                output_index: Some(output_index),
            } => format!("Mantle reasoning output_index {output_index}"),
            Self::MantleReasoning { output_index: None } => {
                "unscoped Mantle reasoning channel".to_string()
            }
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
            pub(crate) fn stop(&mut self);
        }
    }

    /// Parse a Responses SSE event payload and apply source-side output-item
    /// lifecycle validation before returning the typed event.
    pub(crate) fn parse_stream_event(
        &mut self,
        payload: Value,
    ) -> StreamTranslationResult<ResponsesInboundStreamEvent> {
        let mut parsed = if let Some(event) = mantle::parse_stream_event(&payload) {
            ResponsesInboundStreamEvent::Mantle(event?)
        } else {
            ResponsesInboundStreamEvent::Official(Box::new(
                deserialize_value::<ResponseStreamEvent>(
                    &payload,
                    "OpenAI Responses stream event",
                )?,
            ))
        };
        match &mut parsed {
            ResponsesInboundStreamEvent::Official(event) => {
                self.apply_output_item_lifecycle(event)?;
            }
            ResponsesInboundStreamEvent::Mantle(event) => {
                self.observe_mantle_reasoning_event(event)?;
            }
        }
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
            ResponseStreamEvent::ResponseContentPartAdded(event) => {
                self.saw_refusal |= matches!(&event.part, OutputContent::Refusal(_));
                self.observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::from(&event.part),
                    event_type,
                )
            }
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
            ResponseStreamEvent::ResponseRefusalDelta(event) => {
                self.saw_refusal = true;
                self.observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Message,
                    event_type,
                )
            }
            ResponseStreamEvent::ResponseRefusalDone(event) => {
                self.saw_refusal = true;
                self.observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::Message,
                    event_type,
                )
            }
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
            ResponseStreamEvent::ResponseContentPartDone(event) => {
                self.saw_refusal |= matches!(&event.part, OutputContent::Refusal(_));
                self.observe_required_output_item(
                    event.output_index,
                    &event.item_id,
                    ResponsesOutputKind::from(&event.part),
                    event_type,
                )
            }
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
        Self::validate_and_observe_output_item(
            observed,
            output_index,
            Some(item_id),
            expected_kind,
            event,
        )
    }

    /// Validate Mantle's optional official-style identity and assign one stable
    /// projection coordinate to the active compatibility reasoning channel.
    ///
    /// A missing coordinate may inherit an already observed `output_index`, but
    /// a channel that started unscoped stays unscoped: already-forwarded content
    /// cannot be retroactively moved to a keyed target block.
    fn observe_mantle_reasoning_event(
        &mut self,
        event: &mut MantleStreamEvent,
    ) -> StreamTranslationResult<()> {
        let (raw_output_index, item_id) = match &*event {
            MantleStreamEvent::ReasoningDelta {
                output_index,
                item_id,
                ..
            }
            | MantleStreamEvent::ReasoningDone {
                output_index,
                item_id,
                ..
            } => (*output_index, item_id.as_deref()),
        };
        self.require_streaming_output_item_event(event.as_ref())?;
        if let Some(output_index) = raw_output_index
            && let Some(observed) = self.output_items.get_mut(&output_index)
        {
            Self::validate_and_observe_output_item(
                observed,
                output_index,
                item_id,
                ResponsesOutputKind::Reasoning,
                event.as_ref(),
            )?;
        }

        let stable_output_index = match (self.mantle_reasoning, raw_output_index) {
            (MantleReasoningChannel::Inactive, output_index) => output_index,
            (MantleReasoningChannel::Unscoped, _) => None,
            (MantleReasoningChannel::Scoped(expected), Some(actual)) if expected != actual => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream changed Mantle reasoning output_index from {expected} to {actual} before response.reasoning.done"
                )));
            }
            (MantleReasoningChannel::Scoped(expected), _) => Some(expected),
        };

        match event {
            MantleStreamEvent::ReasoningDelta { output_index, .. } => {
                if self.mantle_reasoning == MantleReasoningChannel::Inactive {
                    self.mantle_reasoning = match stable_output_index {
                        Some(output_index) => MantleReasoningChannel::Scoped(output_index),
                        None => MantleReasoningChannel::Unscoped,
                    };
                }
                *output_index = stable_output_index;
            }
            MantleStreamEvent::ReasoningDone { output_index, .. } => {
                *output_index = stable_output_index;
                self.mantle_reasoning = MantleReasoningChannel::Inactive;
            }
        }
        Ok(())
    }

    fn validate_and_observe_output_item(
        observed: &mut ObservedOutputItem,
        output_index: u32,
        item_id: Option<&str>,
        expected_kind: ResponsesOutputKind,
        event: &str,
    ) -> StreamTranslationResult<()> {
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
        if let Some(item_id) = item_id {
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

    /// Infer the terminal source stop semantic from both incrementally observed
    /// output items and the terminal response snapshot. Some compatible
    /// providers omit `response.output` from terminal events, so streaming
    /// observations take precedence over snapshot-only inference.
    pub(crate) fn infer_stop_kind(
        &self,
        response: &Response,
        scope: &TranslationScope,
    ) -> Option<ResponsesStopKind> {
        if self.saw_refusal {
            return Some(ResponsesStopKind::Refusal);
        }
        if self.output_items.values().any(|item| {
            matches!(
                item.kind,
                ResponsesOutputKind::FunctionCall | ResponsesOutputKind::CustomToolCall
            )
        }) {
            return Some(ResponsesStopKind::ToolUse);
        }
        infer_response_stop_kind(response, scope)
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
            .require_streaming_phase_mut("Responses", "active content event")?;
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

    /// Validate that the semantic Responses stream reached a terminal event
    /// and completed target projection before its carrier ended.
    pub(crate) fn finish_stream(&self, end: StreamEnd) -> StreamTranslationResult<()> {
        let message = match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                format!("Responses stream reached {end} before response.created")
            }
            InboundStreamLifecyclePhase::Streaming => {
                format!("Responses stream reached {end} before a terminal response event")
            }
            InboundStreamLifecyclePhase::Terminal => {
                format!(
                    "Responses stream reached {end} after terminal event but before target projection completed"
                )
            }
            InboundStreamLifecyclePhase::Stopped => return Ok(()),
        };
        Err(StreamTranslationError::Semantic(message))
    }
}

#[cfg(test)]
mod tests;
