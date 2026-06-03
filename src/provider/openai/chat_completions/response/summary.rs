use std::collections::BTreeMap;

use serde::Serialize;
use strum::Display;

use crate::protocol::openai::chat_completions::{
    ChatCompletionMessageToolCalls, ChatResponseProjection, ChatStreamResponseProjection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Serialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChatResponseOutputKind {
    Choice,
    FinishedChoice,
    Text,
    Refusal,
    ToolCall,
    CustomToolCall,
    Annotation,
    Audio,
    StreamDelta,
    StreamTextDelta,
    StreamRefusalDelta,
    StreamToolCallDelta,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChatResponseSummary {
    pub(crate) output_items: BTreeMap<ChatResponseOutputKind, u64>,
    pub(crate) finish_reasons: BTreeMap<String, u64>,
    pub(crate) tool_call_names: BTreeMap<String, u64>,
    pub(crate) custom_tool_call_names: BTreeMap<String, u64>,
}

impl From<&ChatResponseProjection> for ChatResponseSummary {
    fn from(projection: &ChatResponseProjection) -> Self {
        let mut summary = Self::default();
        for choice in &projection.choices {
            summary.add_item_kind(ChatResponseOutputKind::Choice);
            if let Some(reason) = choice.finish_reason {
                summary.add_item_kind(ChatResponseOutputKind::FinishedChoice);
                summary.add_finish_reason(reason.to_string());
            }
            if choice.message.content.is_some() {
                summary.add_item_kind(ChatResponseOutputKind::Text);
            }
            if choice.message.refusal.is_some() {
                summary.add_item_kind(ChatResponseOutputKind::Refusal);
            }
            if let Some(tool_calls) = choice.message.tool_calls.as_deref() {
                for tool_call in tool_calls {
                    match tool_call {
                        ChatCompletionMessageToolCalls::Function(tool_call) => {
                            summary.add_tool_call(&tool_call.function.name);
                        }
                        ChatCompletionMessageToolCalls::Custom(tool_call) => {
                            summary.add_custom_tool_call(&tool_call.custom_tool.name);
                        }
                    }
                }
            }
            if let Some(annotations) = choice.message.annotations.as_ref() {
                summary.add_item_kind_count(
                    ChatResponseOutputKind::Annotation,
                    annotations.len() as u64,
                );
            }
            if choice.message.audio.is_some() {
                summary.add_item_kind(ChatResponseOutputKind::Audio);
            }
        }
        summary
    }
}

impl From<&ChatStreamResponseProjection> for ChatResponseSummary {
    fn from(projection: &ChatStreamResponseProjection) -> Self {
        let mut summary = Self::default();
        for choice in &projection.choices {
            summary.add_item_kind(ChatResponseOutputKind::Choice);
            summary.add_item_kind(ChatResponseOutputKind::StreamDelta);
            if let Some(reason) = choice.finish_reason {
                summary.add_item_kind(ChatResponseOutputKind::FinishedChoice);
                summary.add_finish_reason(reason.to_string());
            }
            if choice.delta.content.is_some() {
                summary.add_item_kind(ChatResponseOutputKind::StreamTextDelta);
            }
            if choice.delta.refusal.is_some() {
                summary.add_item_kind(ChatResponseOutputKind::StreamRefusalDelta);
            }
            if let Some(tool_calls) = choice.delta.tool_calls.as_deref() {
                for tool_call in tool_calls {
                    summary.add_item_kind(ChatResponseOutputKind::StreamToolCallDelta);
                    if let Some(function) = tool_call.function.as_ref()
                        && let Some(name) = function.name.as_deref()
                    {
                        summary.add_tool_call_name(name);
                    }
                }
            }
        }
        summary
    }
}

impl ChatResponseSummary {
    pub(crate) fn is_empty(&self) -> bool {
        self.output_items.is_empty()
            && self.finish_reasons.is_empty()
            && self.tool_call_names.is_empty()
            && self.custom_tool_call_names.is_empty()
    }

    pub(crate) fn add_item_kind_count(&mut self, kind: ChatResponseOutputKind, count: u64) {
        *self.output_items.entry(kind).or_default() += count;
    }

    pub(crate) fn add_item_kind(&mut self, kind: ChatResponseOutputKind) {
        self.add_item_kind_count(kind, 1);
    }

    pub(crate) fn add_finish_reason_count(&mut self, reason: &str, count: u64) {
        *self.finish_reasons.entry(reason.to_string()).or_default() += count;
    }

    fn add_finish_reason(&mut self, reason: String) {
        self.add_finish_reason_count(&reason, 1);
    }

    pub(crate) fn add_tool_call(&mut self, name: &str) {
        self.add_item_kind(ChatResponseOutputKind::ToolCall);
        self.add_tool_call_name(name);
    }

    fn add_tool_call_name(&mut self, name: &str) {
        *self.tool_call_names.entry(name.to_string()).or_default() += 1;
    }

    fn add_custom_tool_call(&mut self, name: &str) {
        self.add_item_kind(ChatResponseOutputKind::CustomToolCall);
        *self
            .custom_tool_call_names
            .entry(name.to_string())
            .or_default() += 1;
    }
}
