use std::collections::BTreeMap;

use crate::protocol::openai::chat_completions::{
    ChatResponseProjection, ChatStreamResponseProjection, CompletionUsage, ServiceTier,
};

use super::summary::{ChatResponseOutputKind, ChatResponseSummary};

/// One parsed Chat Completions upstream response shape.
///
/// `NonStream` is a complete JSON response body. `StreamChunk` is a single
/// SSE chunk projection, not a reconstructed full streaming response.
#[derive(Debug, Clone)]
pub(crate) enum ChatResponseObservation {
    NonStream(ChatResponseProjection),
    StreamChunk(ChatStreamResponseProjection),
}

impl Default for ChatResponseObservation {
    fn default() -> Self {
        Self::NonStream(ChatResponseProjection::default())
    }
}

impl ChatResponseObservation {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::NonStream(projection) => &projection.id,
            Self::StreamChunk(projection) => &projection.id,
        }
    }

    pub(crate) fn model(&self) -> &str {
        match self {
            Self::NonStream(projection) => &projection.model,
            Self::StreamChunk(projection) => &projection.model,
        }
    }

    pub(crate) fn service_tier(&self) -> Option<ServiceTier> {
        match self {
            Self::NonStream(projection) => projection.service_tier,
            Self::StreamChunk(projection) => projection.service_tier,
        }
    }

    pub(crate) fn usage(&self) -> Option<&CompletionUsage> {
        match self {
            Self::NonStream(projection) => projection.usage.as_ref(),
            Self::StreamChunk(projection) => projection.usage.as_ref(),
        }
    }

    pub(crate) fn has_finish_reason(&self) -> bool {
        match self {
            Self::NonStream(projection) => projection
                .choices
                .iter()
                .any(|choice| choice.finish_reason.is_some()),
            Self::StreamChunk(projection) => projection
                .choices
                .iter()
                .any(|choice| choice.finish_reason.is_some()),
        }
    }

    pub(crate) fn summary(&self) -> ChatResponseSummary {
        match self {
            Self::NonStream(projection) => ChatResponseSummary::from(projection),
            Self::StreamChunk(projection) => ChatResponseSummary::from(projection),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ObservedToolCallKey {
    choice_index: u32,
    tool_index: u32,
}

#[derive(Debug, Clone, Default)]
struct ObservedChoice {
    has_text: bool,
    has_refusal: bool,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ObservedToolCall {
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum ObservedChatUpdate {
    Choice {
        index: u32,
    },
    Text {
        index: u32,
    },
    Refusal {
        index: u32,
    },
    ToolCall {
        choice_index: u32,
        tool_index: u32,
        name: Option<String>,
    },
    FinishReason {
        index: u32,
        reason: String,
    },
}

pub(crate) fn observed_updates_from_stream_projection(
    projection: &ChatStreamResponseProjection,
) -> Vec<ObservedChatUpdate> {
    let mut updates = Vec::new();
    for choice in &projection.choices {
        updates.push(ObservedChatUpdate::Choice {
            index: choice.index,
        });
        if choice.delta.content.is_some() {
            updates.push(ObservedChatUpdate::Text {
                index: choice.index,
            });
        }
        if choice.delta.refusal.is_some() {
            updates.push(ObservedChatUpdate::Refusal {
                index: choice.index,
            });
        }
        if let Some(tool_calls) = choice.delta.tool_calls.as_deref() {
            for tool_call in tool_calls {
                updates.push(ObservedChatUpdate::ToolCall {
                    choice_index: choice.index,
                    tool_index: tool_call.index,
                    name: tool_call
                        .function
                        .as_ref()
                        .and_then(|function| function.name.clone()),
                });
            }
        }
        if let Some(reason) = choice.finish_reason {
            updates.push(ObservedChatUpdate::FinishReason {
                index: choice.index,
                reason: reason.to_string(),
            });
        }
    }
    updates
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ObservedChatState {
    choices: BTreeMap<u32, ObservedChoice>,
    tool_calls: BTreeMap<ObservedToolCallKey, ObservedToolCall>,
}

impl ObservedChatState {
    pub(crate) fn apply(&mut self, update: &ObservedChatUpdate) {
        match update {
            ObservedChatUpdate::Choice { index } => {
                self.choices.entry(*index).or_default();
            }
            ObservedChatUpdate::Text { index } => {
                self.choices.entry(*index).or_default().has_text = true;
            }
            ObservedChatUpdate::Refusal { index } => {
                self.choices.entry(*index).or_default().has_refusal = true;
            }
            ObservedChatUpdate::ToolCall {
                choice_index,
                tool_index,
                name,
            } => {
                self.choices.entry(*choice_index).or_default();
                let key = ObservedToolCallKey {
                    choice_index: *choice_index,
                    tool_index: *tool_index,
                };
                let tool_call = self.tool_calls.entry(key).or_default();
                if let Some(name) = name {
                    tool_call.name.get_or_insert_with(|| name.clone());
                }
            }
            ObservedChatUpdate::FinishReason { index, reason } => {
                self.choices
                    .entry(*index)
                    .or_default()
                    .finish_reason
                    .get_or_insert_with(|| reason.clone());
            }
        }
    }

    pub(crate) fn fallback_summary(&self) -> ChatResponseSummary {
        let mut summary = ChatResponseSummary::default();
        for choice in self.choices.values() {
            summary.add_item_kind(ChatResponseOutputKind::Choice);
            if let Some(reason) = &choice.finish_reason {
                summary.add_item_kind(ChatResponseOutputKind::FinishedChoice);
                summary.add_finish_reason_count(reason, 1);
            }
            if choice.has_text {
                summary.add_item_kind(ChatResponseOutputKind::Text);
            }
            if choice.has_refusal {
                summary.add_item_kind(ChatResponseOutputKind::Refusal);
            }
        }

        for tool_call in self.tool_calls.values() {
            if let Some(name) = &tool_call.name {
                summary.add_tool_call(name);
            } else {
                summary.add_item_kind(ChatResponseOutputKind::ToolCall);
            }
        }
        summary
    }
}

#[cfg(test)]
#[path = "observed_tests.rs"]
mod tests;
