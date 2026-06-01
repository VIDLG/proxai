use std::collections::{BTreeMap, BTreeSet};

use crate::protocol::openai::chat_completions::{ChatCompletionTools, RequestProjection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ToolCategory {
    Function,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolInventoryItem {
    pub(crate) category: ToolCategory,
    pub(crate) count: usize,
    pub(crate) names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RequestSummary {
    pub(crate) tool_inventory: Vec<ToolInventoryItem>,
}

impl From<&RequestProjection> for RequestSummary {
    fn from(projection: &RequestProjection) -> Self {
        Self {
            tool_inventory: extract_tool_inventory(projection),
        }
    }
}

fn extract_tool_inventory(projection: &RequestProjection) -> Vec<ToolInventoryItem> {
    let mut grouped: BTreeMap<ToolCategory, BTreeSet<String>> = BTreeMap::new();
    let mut counts: BTreeMap<ToolCategory, usize> = BTreeMap::new();

    if let Some(items) = projection.tools.as_deref() {
        for item in items {
            let (category, names) = match item {
                ChatCompletionTools::Function(tool) => {
                    (ToolCategory::Function, vec![tool.function.name.clone()])
                }
                ChatCompletionTools::Custom(tool) => {
                    (ToolCategory::Custom, vec![tool.custom.name.clone()])
                }
            };
            *counts.entry(category).or_default() += 1;
            for name in names {
                grouped.entry(category).or_default().insert(name);
            }
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
