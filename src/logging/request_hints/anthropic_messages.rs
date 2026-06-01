use crate::protocol::anthropic::messages::{
    MessageCreateParamsBase, RequestServiceTier, ThinkingConfigParam, ToolChoice,
};
use crate::provider::anthropic_messages::request::{RequestSummary, ToolCategory};

pub(crate) fn render_summary_compact(
    projection: &MessageCreateParamsBase,
    summary: &RequestSummary,
) -> Vec<String> {
    let mut parts = Vec::new();
    if let Some(tool_choice) = projection.tool_choice.as_ref() {
        parts.push(render_tool_choice_compact(tool_choice));
    }
    if let Some(thinking) = projection.thinking.as_ref() {
        parts.push(render_thinking_compact(thinking));
    }
    parts.extend(super::render_tool_inventory(
        summary.tool_inventory.iter().map(|item| {
            (
                render_tool_category_compact(item.category),
                item.count,
                item.names.as_slice(),
            )
        }),
    ));
    parts
}

pub(crate) fn render_projection_compact(projection: &MessageCreateParamsBase) -> String {
    let mut parts = vec!["Anthropic/passthrough".to_string()];
    if projection.container.is_some() {
        parts.push("container".to_string());
    }
    if projection.metadata.is_some() {
        parts.push("meta".to_string());
    }
    if let Some(service_tier) = projection.service_tier {
        parts.push(format!(
            "tier:{}",
            render_service_tier_compact(service_tier)
        ));
    }
    if projection.output_config.is_some() {
        parts.push("output_config".to_string());
    }
    if projection.system.is_some() {
        parts.push("system".to_string());
    }
    if let Some(value) = projection.temperature.as_ref() {
        parts.push(format!("temp:{value}"));
    }
    if let Some(value) = projection.top_k {
        parts.push(format!("top_k:{value}"));
    }
    if let Some(value) = projection.top_p.as_ref() {
        parts.push(format!("top_p:{value}"));
    }
    if projection
        .stop_sequences
        .as_ref()
        .is_some_and(|value| !value.is_empty())
    {
        parts.push("stop".to_string());
    }
    parts.join(" ")
}

fn render_tool_category_compact(category: ToolCategory) -> &'static str {
    match category {
        ToolCategory::Custom => "c",
        ToolCategory::Bash => "bash",
        ToolCategory::CodeExecution => "code",
        ToolCategory::Memory => "mem",
        ToolCategory::TextEditor => "edit",
        ToolCategory::WebFetch => "web_fetch",
        ToolCategory::WebSearch => "web",
        ToolCategory::ToolSearch => "tool_search",
    }
}

fn render_tool_choice_compact(tool_choice: &ToolChoice) -> String {
    match tool_choice {
        ToolChoice::Auto(_) => "tc:auto".to_string(),
        ToolChoice::Any(_) => "tc:any".to_string(),
        ToolChoice::Tool(tool) => format!("tc:tool:{}", tool.name),
        ToolChoice::None(_) => "tc:none".to_string(),
    }
}

fn render_thinking_compact(thinking: &ThinkingConfigParam) -> String {
    match thinking {
        ThinkingConfigParam::Enabled(value) => format!("think:{}", value.budget_tokens),
        ThinkingConfigParam::Adaptive(_) => "think:adaptive".to_string(),
        ThinkingConfigParam::Disabled(_) => "think:disabled".to_string(),
    }
}

fn render_service_tier_compact(service_tier: RequestServiceTier) -> &'static str {
    match service_tier {
        RequestServiceTier::Auto => "auto",
        RequestServiceTier::StandardOnly => "standard_only",
    }
}
