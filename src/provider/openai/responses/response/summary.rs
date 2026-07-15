use std::collections::BTreeMap;

use serde::Serialize;

use crate::protocol::openai_responses::{OutputItem, OutputItemKind, ResponseProjection};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct ResponseSummary {
    pub(crate) output_items: BTreeMap<OutputItemKind, u64>,
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
    pub(crate) fn is_empty(&self) -> bool {
        self.output_items.is_empty() && self.function_calls.is_empty() && self.mcp_calls.is_empty()
    }

    /// Records a concrete output item variant and any associated name-level
    /// summaries derived from that item.
    pub(crate) fn record_output_item(&mut self, item: &OutputItem) {
        match item {
            OutputItem::FunctionCall(item) => self.add_function_call_item(&item.name),
            OutputItem::McpCall(item) => self.add_mcp_call_item(&item.server_label, &item.name),
            item => self.add_item_kind(item.into()),
        }
    }

    pub(crate) fn add_item_kind_count(&mut self, kind: OutputItemKind, count: u64) {
        *self.output_items.entry(kind).or_default() += count;
    }

    pub(crate) fn add_item_kind(&mut self, kind: OutputItemKind) {
        self.add_item_kind_count(kind, 1);
    }

    pub(crate) fn add_function_call_item(&mut self, name: &str) {
        self.add_item_kind(OutputItemKind::FunctionCall);
        self.add_function_call_name(name);
    }

    fn add_function_call_name(&mut self, name: &str) {
        *self.function_calls.entry(name.to_string()).or_default() += 1;
    }

    pub(crate) fn add_mcp_call_item(&mut self, server_label: &str, name: &str) {
        self.add_item_kind(OutputItemKind::McpCall);
        self.add_mcp_call_name(server_label, name);
    }

    fn add_mcp_call_name(&mut self, server_label: &str, name: &str) {
        *self
            .mcp_calls
            .entry(format!("{server_label}/{name}"))
            .or_default() += 1;
    }
}
