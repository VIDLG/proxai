use crate::protocol::openai::chat_completions::{
    self, ChatCompletionToolChoiceOption, RequestProjection,
};
use crate::provider::openai::chat_completions::{RequestSummary, ToolCategory};

pub(crate) fn render_summary_compact(summary: &RequestSummary) -> Vec<String> {
    super::render_tool_inventory(summary.tool_inventory.iter().map(|item| {
        (
            render_tool_category_compact(item.category),
            item.count,
            item.names.as_slice(),
        )
    }))
}

pub(crate) fn render_projection_compact(projection: &RequestProjection) -> String {
    let mut parts = Vec::new();
    if projection.parallel_tool_calls == Some(true) {
        parts.push("ptc".to_string());
    }
    if projection.store == Some(true) {
        parts.push("store".to_string());
    }
    if projection.prompt_cache_key.is_some() {
        parts.push("pck".to_string());
    }
    if projection.metadata.is_some() {
        parts.push("meta".to_string());
    }
    if projection.safety_identifier.is_some() {
        parts.push("sid".to_string());
    }
    if let Some(value) = projection.temperature {
        parts.push(format!("temp:{value}"));
    }
    if let Some(value) = projection.top_p {
        parts.push(format!("top_p:{value}"));
    }
    if let Some(value) = projection.top_logprobs {
        parts.push(format!("tlp:{value}"));
    }
    if projection.logprobs == Some(true) {
        parts.push("logprobs".to_string());
    }
    if let Some(value) = projection.n
        && value != 1
    {
        parts.push(format!("n:{value}"));
    }
    if projection
        .stream_options
        .as_ref()
        .and_then(|options| options.include_usage)
        == Some(true)
    {
        parts.push("so:usage".to_string());
    }
    if projection
        .stream_options
        .as_ref()
        .and_then(|options| options.include_obfuscation)
        == Some(false)
    {
        parts.push("so:no-obf".to_string());
    }
    if let Some(value) = projection.service_tier
        && !matches!(
            value,
            chat_completions::ServiceTier::Auto | chat_completions::ServiceTier::Default
        )
    {
        parts.push(format!("tier:{value}"));
    }
    if let Some(value) = projection.verbosity {
        parts.push(format!("v:{value}"));
    }
    if let Some(choice) = projection.tool_choice.as_ref() {
        parts.push(render_tool_choice_compact(choice));
    }
    if let Some(format) = projection.response_format.as_ref() {
        parts.push(match format {
            chat_completions::ResponseFormat::Text => "rf:text".to_string(),
            chat_completions::ResponseFormat::JsonObject => "rf:json_object".to_string(),
            chat_completions::ResponseFormat::JsonSchema { .. } => "rf:json_schema".to_string(),
        });
    }
    parts.join(" ")
}

fn render_tool_category_compact(category: ToolCategory) -> &'static str {
    match category {
        ToolCategory::Function => "f",
        ToolCategory::Custom => "c",
    }
}

fn render_tool_choice_compact(choice: &ChatCompletionToolChoiceOption) -> String {
    match choice {
        ChatCompletionToolChoiceOption::Mode(mode) => match mode {
            chat_completions::ToolChoiceOptions::Auto => "tc:auto".to_string(),
            chat_completions::ToolChoiceOptions::None => "tc:none".to_string(),
            chat_completions::ToolChoiceOptions::Required => "tc:required".to_string(),
        },
        ChatCompletionToolChoiceOption::Function(choice) => {
            format!("tc:function:{}", choice.function.name)
        }
        ChatCompletionToolChoiceOption::Custom(choice) => {
            format!("tc:custom:{}", choice.custom.name)
        }
        ChatCompletionToolChoiceOption::AllowedTools(choice) => {
            format!("tc:allowed_tools:{}", choice.allowed_tools.len())
        }
    }
}
