use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::openai_responses::{
    MCPTool, MCPToolAllowedTools, MCPToolFilter, RequestProjection, Tool,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ToolCategory {
    Function,
    Mcp,
    Custom,
    WebSearch,
    FileSearch,
    Computer,
    CodeInterpreter,
    ImageGeneration,
    Shell,
    ApplyPatch,
    Namespace,
    ToolSearch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInventoryItem {
    pub(crate) category: ToolCategory,
    /// Total number of tool declarations in this category, including duplicate
    /// names and built-in tools that do not expose a stable display name.
    pub(crate) count: usize,
    /// Distinct display names we can extract for this category. This can be
    /// smaller than `count` when names repeat or built-in tools are unnamed.
    pub(crate) names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RequestSummary {
    pub(crate) tool_inventory: Vec<ToolInventoryItem>,
}

impl From<&RequestProjection> for RequestSummary {
    fn from(projection: &RequestProjection) -> Self {
        Self {
            tool_inventory: extract_tool_inventory(projection.tools.as_deref()),
        }
    }
}

fn extract_tool_inventory(items: Option<&[Tool]>) -> Vec<ToolInventoryItem> {
    let Some(items) = items else {
        return Vec::new();
    };
    let mut grouped: BTreeMap<ToolCategory, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<ToolCategory, usize> = BTreeMap::new();

    for item in items {
        let (category, names) = match item {
            Tool::Function(tool) => (ToolCategory::Function, vec![tool.name.clone()]),
            Tool::FileSearch(_) => (ToolCategory::FileSearch, Vec::new()),
            Tool::ComputerUsePreview(_) => (ToolCategory::Computer, Vec::new()),
            Tool::WebSearch(_) => (ToolCategory::WebSearch, Vec::new()),
            Tool::WebSearch20250826(_) => (ToolCategory::WebSearch, Vec::new()),
            Tool::Mcp(tool) => (ToolCategory::Mcp, mcp_tool_names(tool)),
            Tool::CodeInterpreter(_) => (ToolCategory::CodeInterpreter, Vec::new()),
            Tool::ImageGeneration(_) => (ToolCategory::ImageGeneration, Vec::new()),
            Tool::LocalShell => (ToolCategory::Shell, Vec::new()),
            Tool::Shell(_) => (ToolCategory::Shell, Vec::new()),
            Tool::Custom(tool) => (ToolCategory::Custom, vec![tool.name.clone()]),
            Tool::Computer(_) => (ToolCategory::Computer, Vec::new()),
            Tool::Namespace(tool) => (ToolCategory::Namespace, vec![tool.name.clone()]),
            Tool::ToolSearch(_) => (ToolCategory::ToolSearch, Vec::new()),
            Tool::WebSearchPreview(_) => (ToolCategory::WebSearch, Vec::new()),
            Tool::WebSearchPreview20250311(_) => (ToolCategory::WebSearch, Vec::new()),
            Tool::ApplyPatch => (ToolCategory::ApplyPatch, Vec::new()),
        };
        *counts.entry(category).or_default() += 1;
        for name in names {
            grouped.entry(category).or_default().insert(name);
        }
    }

    counts
        .into_iter()
        .map(|(category, count)| {
            let names = grouped
                .get(&category)
                .into_iter()
                .flatten()
                .cloned()
                .collect();
            ToolInventoryItem {
                category,
                count,
                names,
            }
        })
        .collect()
}

fn mcp_tool_names(tool: &MCPTool) -> Vec<String> {
    match tool.allowed_tools.as_ref() {
        Some(MCPToolAllowedTools::List(names)) => {
            prefixed_mcp_tool_names(&tool.server_label, names)
        }
        Some(MCPToolAllowedTools::Filter(filter)) => {
            mcp_filter_tool_names(&tool.server_label, filter)
        }
        None => vec![tool.server_label.clone()],
    }
}

fn mcp_filter_tool_names(server_label: &str, filter: &MCPToolFilter) -> Vec<String> {
    if let Some(names) = filter.tool_names.as_deref() {
        return prefixed_mcp_tool_names(server_label, names);
    }

    match filter.read_only {
        Some(true) => vec![format!("{server_label}[ro]")],
        Some(false) => vec![format!("{server_label}[rw]")],
        None => vec![server_label.to_string()],
    }
}

fn prefixed_mcp_tool_names(server_label: &str, names: &[String]) -> Vec<String> {
    if names.is_empty() {
        return vec![server_label.to_string()];
    }

    names
        .iter()
        .map(|name| format!("{server_label}/{name}"))
        .collect()
}
