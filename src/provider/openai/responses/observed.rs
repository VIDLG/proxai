use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::openai_responses::OutputItem;
use crate::protocol::ErrorObject;

use super::{ResponseOutputItemKind, ResponseSummary};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ObservedEntityId(String);

#[derive(Debug, Clone)]
enum ObservedEntity {
    Message,
    FunctionCall {
        name: Option<String>,
    },
    Reasoning,
    McpCall {
        server_label: Option<String>,
        name: Option<String>,
    },
    McpListTools,
}

#[derive(Debug, Clone)]
pub(super) enum ObservedUpdate {
    Message {
        id: String,
    },
    FunctionCall {
        id: String,
        name: String,
    },
    Reasoning {
        id: String,
    },
    McpCall {
        id: String,
        server_label: Option<String>,
        name: Option<String>,
    },
    McpListTools {
        id: String,
    },
    SummaryOnlyItemKind {
        kind: ResponseOutputItemKind,
        event_key: String,
    },
}

impl ObservedUpdate {
    pub(super) fn from_output_item(item: &OutputItem, output_index: u32) -> Self {
        match item {
            OutputItem::Message(item) => Self::Message {
                id: item.id.clone(),
            },
            OutputItem::FunctionCall(item) => item.id.as_ref().map_or_else(
                || {
                    Self::from_summary_only_item_kind(
                        ResponseOutputItemKind::FunctionCall,
                        output_index,
                    )
                },
                |id| Self::FunctionCall {
                    id: id.clone(),
                    name: item.name.clone(),
                },
            ),
            OutputItem::Reasoning(item) => item.id.as_ref().map_or_else(
                || {
                    Self::from_summary_only_item_kind(
                        ResponseOutputItemKind::Reasoning,
                        output_index,
                    )
                },
                |id| Self::Reasoning { id: id.clone() },
            ),
            OutputItem::FunctionCallOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::FunctionCallOutput,
                output_index,
            ),
            OutputItem::FileSearchCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::FileSearchCall,
                output_index,
            ),
            OutputItem::WebSearchCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::WebSearchCall,
                output_index,
            ),
            OutputItem::ComputerCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ComputerCall,
                output_index,
            ),
            OutputItem::ComputerCallOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ComputerCallOutput,
                output_index,
            ),
            OutputItem::ImageGenerationCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ImageGenerationCall,
                output_index,
            ),
            OutputItem::CodeInterpreterCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::CodeInterpreterCall,
                output_index,
            ),
            OutputItem::LocalShellCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::LocalShellCall,
                output_index,
            ),
            OutputItem::ShellCall(_) => {
                Self::from_summary_only_item_kind(ResponseOutputItemKind::ShellCall, output_index)
            }
            OutputItem::ShellCallOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ShellCallOutput,
                output_index,
            ),
            OutputItem::ApplyPatchCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ApplyPatchCall,
                output_index,
            ),
            OutputItem::ApplyPatchCallOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ApplyPatchCallOutput,
                output_index,
            ),
            OutputItem::McpCall(item) => Self::McpCall {
                id: item.id.clone(),
                server_label: Some(item.server_label.clone()),
                name: Some(item.name.clone()),
            },
            OutputItem::McpListTools(item) => Self::McpListTools {
                id: item.id.clone(),
            },
            OutputItem::McpApprovalRequest(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::McpApprovalRequest,
                output_index,
            ),
            OutputItem::CustomToolCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::CustomToolCall,
                output_index,
            ),
            OutputItem::CustomToolCallOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::CustomToolCallOutput,
                output_index,
            ),
            OutputItem::ToolSearchCall(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ToolSearchCall,
                output_index,
            ),
            OutputItem::ToolSearchOutput(_) => Self::from_summary_only_item_kind(
                ResponseOutputItemKind::ToolSearchOutput,
                output_index,
            ),
            OutputItem::Compaction(_) => {
                Self::from_summary_only_item_kind(ResponseOutputItemKind::Compaction, output_index)
            }
        }
    }

    fn from_summary_only_item_kind(kind: ResponseOutputItemKind, output_index: u32) -> Self {
        Self::SummaryOnlyItemKind {
            kind,
            event_key: format!("{kind}:{output_index}"),
        }
    }

    pub(super) fn from_function_call_arguments_done(item_id: &str, name: &str) -> Self {
        Self::FunctionCall {
            id: item_id.to_string(),
            name: name.to_string(),
        }
    }

    pub(super) fn from_mcp_call_lifecycle(item_id: &str) -> Self {
        Self::McpCall {
            id: item_id.to_string(),
            server_label: None,
            name: None,
        }
    }

    pub(super) fn from_mcp_list_tools_lifecycle(item_id: &str) -> Self {
        Self::McpListTools {
            id: item_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct ObservedState {
    entities: BTreeMap<ObservedEntityId, ObservedEntity>,
    anonymous_item_keys: BTreeSet<ObservedEntityId>,
    anonymous_item_kinds: BTreeMap<ResponseOutputItemKind, u64>,
    error: Option<ErrorObject>,
}

impl ObservedState {
    pub(super) fn apply(&mut self, update: &ObservedUpdate) {
        match update {
            ObservedUpdate::Message { id } => {
                self.entities
                    .entry(ObservedEntityId(id.clone()))
                    .or_insert(ObservedEntity::Message);
            }
            ObservedUpdate::FunctionCall { id, name } => {
                let entity = self
                    .entities
                    .entry(ObservedEntityId(id.clone()))
                    .or_insert_with(|| ObservedEntity::FunctionCall { name: None });
                if let ObservedEntity::FunctionCall { name: entity_name } = entity {
                    entity_name.get_or_insert_with(|| name.clone());
                }
            }
            ObservedUpdate::Reasoning { id } => {
                self.entities
                    .entry(ObservedEntityId(id.clone()))
                    .or_insert(ObservedEntity::Reasoning);
            }
            ObservedUpdate::McpCall {
                id,
                server_label,
                name,
            } => {
                let entity = self
                    .entities
                    .entry(ObservedEntityId(id.clone()))
                    .or_insert_with(|| ObservedEntity::McpCall {
                        server_label: None,
                        name: None,
                    });
                if let ObservedEntity::McpCall {
                    server_label: entity_server_label,
                    name: entity_name,
                } = entity
                {
                    if let Some(server_label) = server_label {
                        entity_server_label.get_or_insert_with(|| server_label.clone());
                    }
                    if let Some(name) = name {
                        entity_name.get_or_insert_with(|| name.clone());
                    }
                }
            }
            ObservedUpdate::McpListTools { id } => {
                self.entities
                    .entry(ObservedEntityId(id.clone()))
                    .or_insert(ObservedEntity::McpListTools);
            }
            ObservedUpdate::SummaryOnlyItemKind { kind, event_key } => {
                if self
                    .anonymous_item_keys
                    .insert(ObservedEntityId(event_key.clone()))
                {
                    *self.anonymous_item_kinds.entry(*kind).or_default() += 1;
                }
            }
        }
    }

    pub(super) fn record_error(&mut self, error: ErrorObject) {
        self.error = Some(error);
    }

    pub(super) fn error(&self) -> Option<&ErrorObject> {
        self.error.as_ref()
    }
}

impl From<&ObservedState> for ResponseSummary {
    fn from(state: &ObservedState) -> Self {
        let mut summary = Self::default();

        for (kind, count) in &state.anonymous_item_kinds {
            summary.add_item_kind_count(*kind, *count);
        }

        for entity in state.entities.values() {
            match entity {
                ObservedEntity::Message => {
                    summary.add_item_kind(ResponseOutputItemKind::Message);
                }
                ObservedEntity::FunctionCall { name } => {
                    if let Some(name) = name {
                        summary.add_function_call_item(name);
                    } else {
                        summary.add_item_kind(ResponseOutputItemKind::FunctionCall);
                    }
                }
                ObservedEntity::Reasoning => {
                    summary.add_item_kind(ResponseOutputItemKind::Reasoning);
                }
                ObservedEntity::McpCall { server_label, name } => {
                    if let (Some(server_label), Some(name)) = (server_label.as_ref(), name.as_ref())
                    {
                        summary.add_mcp_call_item(server_label, name);
                    } else {
                        summary.add_item_kind(ResponseOutputItemKind::McpCall);
                    }
                }
                ObservedEntity::McpListTools => {
                    summary.add_item_kind(ResponseOutputItemKind::McpListTools);
                }
            }
        }

        summary
    }
}

#[cfg(test)]
#[path = "observed_tests.rs"]
mod tests;
