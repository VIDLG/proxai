use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::anthropic::messages::{MessageCreateParamsBase, ToolUnion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ToolCategory {
    Custom,
    Bash,
    CodeExecution,
    Memory,
    TextEditor,
    WebFetch,
    WebSearch,
    ToolSearch,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RequestSummary {
    pub(crate) tool_inventory: Vec<ToolInventoryItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolInventoryItem {
    pub(crate) category: ToolCategory,
    /// Total number of tool declarations in this category, including duplicate
    /// names and built-in tools that do not expose a stable display name.
    pub(crate) count: usize,
    /// Distinct display names we can extract for this category. This can be
    /// smaller than `count` when names repeat or built-in tools are unnamed.
    pub(crate) names: Vec<String>,
}

impl From<&MessageCreateParamsBase> for RequestSummary {
    fn from(params: &MessageCreateParamsBase) -> Self {
        Self {
            tool_inventory: summarize_tools(params.tools.as_deref()),
        }
    }
}

fn summarize_tools(tools: Option<&[ToolUnion]>) -> Vec<ToolInventoryItem> {
    let mut grouped: BTreeMap<ToolCategory, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<ToolCategory, usize> = BTreeMap::new();

    for tool in tools.unwrap_or_default() {
        let category = tool_category(tool);
        *counts.entry(category).or_default() += 1;

        if let Some(name) = tool_name(tool) {
            grouped
                .entry(category)
                .or_default()
                .insert(name.to_string());
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

fn tool_category(tool: &ToolUnion) -> ToolCategory {
    match tool {
        ToolUnion::Custom(_) => ToolCategory::Custom,
        ToolUnion::ToolBash20250124(_) => ToolCategory::Bash,
        ToolUnion::CodeExecutionTool20250522(_)
        | ToolUnion::CodeExecutionTool20250825(_)
        | ToolUnion::CodeExecutionTool20260120(_) => ToolCategory::CodeExecution,
        ToolUnion::MemoryTool20250818(_) => ToolCategory::Memory,
        ToolUnion::ToolTextEditor20250124(_)
        | ToolUnion::ToolTextEditor20250429(_)
        | ToolUnion::ToolTextEditor20250728(_) => ToolCategory::TextEditor,
        ToolUnion::WebFetchTool20250910(_)
        | ToolUnion::WebFetchTool20260209(_)
        | ToolUnion::WebFetchTool20260309(_) => ToolCategory::WebFetch,
        ToolUnion::WebSearchTool20250305(_) | ToolUnion::WebSearchTool20260209(_) => {
            ToolCategory::WebSearch
        }
        ToolUnion::ToolSearchToolBm25_20251119(_) | ToolUnion::ToolSearchToolRegex20251119(_) => {
            ToolCategory::ToolSearch
        }
    }
}

fn tool_name(tool: &ToolUnion) -> Option<&str> {
    match tool {
        ToolUnion::Custom(tool) => Some(tool.name.as_str()),
        _ => None,
    }
}
