use std::collections::BTreeMap;

use serde::Serialize;
use strum::Display;

use crate::protocol::openai_responses::{OutputItem, ResponseProjection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Serialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum ResponseOutputItemKind {
    Message,
    FunctionCall,
    FunctionCallOutput,
    Reasoning,
    FileSearchCall,
    WebSearchCall,
    ComputerCall,
    ComputerCallOutput,
    ImageGenerationCall,
    CodeInterpreterCall,
    LocalShellCall,
    ShellCall,
    ShellCallOutput,
    ApplyPatchCall,
    ApplyPatchCallOutput,
    McpCall,
    McpListTools,
    McpApprovalRequest,
    CustomToolCall,
    CustomToolCallOutput,
    ToolSearchCall,
    ToolSearchOutput,
    Compaction,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct ResponseSummary {
    pub(crate) output_items: BTreeMap<ResponseOutputItemKind, u64>,
    pub(crate) function_calls: BTreeMap<String, u64>,
    pub(crate) mcp_calls: BTreeMap<String, u64>,
}

impl From<&ResponseProjection> for ResponseSummary {
    fn from(projection: &ResponseProjection) -> Self {
        let mut value = Self::default();
        for item in &projection.output {
            value.record_output_item(item);
        }
        value
    }
}

impl ResponseSummary {
    /// Records a concrete output item variant and any associated name-level
    /// summaries derived from that item.
    pub(crate) fn record_output_item(&mut self, item: &OutputItem) {
        match item {
            OutputItem::Message(_) => self.add_item_kind(ResponseOutputItemKind::Message),
            OutputItem::FunctionCall(item) => self.add_function_call_item(&item.name),
            OutputItem::Reasoning(_) => self.add_item_kind(ResponseOutputItemKind::Reasoning),
            OutputItem::FunctionCallOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::FunctionCallOutput)
            }
            OutputItem::FileSearchCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::FileSearchCall)
            }
            OutputItem::WebSearchCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::WebSearchCall)
            }
            OutputItem::ComputerCall(_) => self.add_item_kind(ResponseOutputItemKind::ComputerCall),
            OutputItem::ComputerCallOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::ComputerCallOutput)
            }
            OutputItem::ImageGenerationCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::ImageGenerationCall)
            }
            OutputItem::CodeInterpreterCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::CodeInterpreterCall)
            }
            OutputItem::LocalShellCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::LocalShellCall)
            }
            OutputItem::ShellCall(_) => self.add_item_kind(ResponseOutputItemKind::ShellCall),
            OutputItem::ShellCallOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::ShellCallOutput)
            }
            OutputItem::ApplyPatchCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::ApplyPatchCall)
            }
            OutputItem::ApplyPatchCallOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::ApplyPatchCallOutput)
            }
            OutputItem::McpCall(item) => self.add_mcp_call_item(&item.server_label, &item.name),
            OutputItem::McpListTools(_) => self.add_item_kind(ResponseOutputItemKind::McpListTools),
            OutputItem::McpApprovalRequest(_) => {
                self.add_item_kind(ResponseOutputItemKind::McpApprovalRequest)
            }
            OutputItem::CustomToolCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::CustomToolCall)
            }
            OutputItem::CustomToolCallOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::CustomToolCallOutput)
            }
            OutputItem::ToolSearchCall(_) => {
                self.add_item_kind(ResponseOutputItemKind::ToolSearchCall)
            }
            OutputItem::ToolSearchOutput(_) => {
                self.add_item_kind(ResponseOutputItemKind::ToolSearchOutput)
            }
            OutputItem::Compaction(_) => self.add_item_kind(ResponseOutputItemKind::Compaction),
        }
    }

    pub(crate) fn add_item_kind_count(&mut self, kind: ResponseOutputItemKind, count: u64) {
        *self.output_items.entry(kind).or_default() += count;
    }

    pub(crate) fn add_item_kind(&mut self, kind: ResponseOutputItemKind) {
        self.add_item_kind_count(kind, 1);
    }

    pub(crate) fn add_function_call_item(&mut self, name: &str) {
        self.add_item_kind(ResponseOutputItemKind::FunctionCall);
        self.add_function_call_name(name);
    }

    fn add_function_call_name(&mut self, name: &str) {
        *self.function_calls.entry(name.to_string()).or_default() += 1;
    }

    pub(crate) fn add_mcp_call_item(&mut self, server_label: &str, name: &str) {
        self.add_item_kind(ResponseOutputItemKind::McpCall);
        self.add_mcp_call_name(server_label, name);
    }

    fn add_mcp_call_name(&mut self, server_label: &str, name: &str) {
        *self
            .mcp_calls
            .entry(format!("{server_label}/{name}"))
            .or_default() += 1;
    }
}
