use crate::formatting::compact_tail;
use crate::protocol::openai::responses::{
    ReasoningSummary, TextResponseFormatConfiguration, ToolChoiceOptions, ToolChoiceParam,
};
use crate::protocol::openai_responses::{IncludeEnum, RequestProjection};
use crate::provider::openai::responses::{RequestSummary, ToolCategory};

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

    if projection.background == Some(true) {
        parts.push("bg".to_string());
    }
    if projection.conversation.is_some() {
        parts.push("conv".to_string());
    }
    if projection.parallel_tool_calls == Some(true) {
        parts.push("ptc".to_string());
    }
    if projection.store == Some(true) {
        parts.push("store".to_string());
    }
    if let Some(value) = projection.max_tool_calls {
        parts.push(format!("mtc:{value}"));
    }
    if projection.prompt_cache_key.is_some() {
        parts.push("pck".to_string());
    }
    if projection.prompt.is_some() {
        parts.push("prompt".to_string());
    }
    if let Some(value) = projection.prompt_cache_retention {
        parts.push(format!("pcr:{value}"));
    }
    if projection.instructions.is_some() {
        parts.push("instr".to_string());
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
    if let Some(value) = projection.truncation {
        parts.push(format!("tr:{value}"));
    }
    if let Some(stream_options) = projection.stream_options.as_ref() {
        if stream_options.include_obfuscation == Some(false) {
            parts.push("so:no-obf".to_string());
        }
    }
    if let Some(value) = projection.service_tier {
        if !matches!(
            value,
            crate::protocol::openai::responses::ServiceTier::Auto
                | crate::protocol::openai::responses::ServiceTier::Default
        ) {
            parts.push(format!("tier:{value}"));
        }
    }
    if let Some(value) = projection
        .reasoning
        .as_ref()
        .and_then(|reasoning| reasoning.summary)
    {
        if !matches!(value, ReasoningSummary::Auto) {
            parts.push(format!("rs:{value}"));
        }
    }
    if let Some(value) = projection.previous_response_id.as_deref() {
        parts.push(format!("prev={}", compact_tail(value, 8)));
    }
    if let Some(tool_choice) = projection.tool_choice.as_ref() {
        parts.push(render_tool_choice_compact(tool_choice));
    }
    if let Some(include) = projection
        .include
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        parts.push(format!(
            "inc[{}]",
            include
                .iter()
                .map(render_include_hint)
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if let Some(value) = projection.text.as_ref().and_then(|text| text.verbosity) {
        parts.push(format!("tv:{value}"));
    }
    if let Some(value) = projection.text.as_ref().map(|text| &text.format) {
        if !matches!(value, TextResponseFormatConfiguration::Text) {
            parts.push(format!("tf:{value}"));
        }
    }

    parts.join(" ")
}

fn render_tool_category_compact(category: ToolCategory) -> &'static str {
    match category {
        ToolCategory::Function => "f",
        ToolCategory::Mcp => "mcp",
        ToolCategory::Custom => "c",
        ToolCategory::WebSearch => "web",
        ToolCategory::FileSearch => "fs",
        ToolCategory::Computer => "pc",
        ToolCategory::CodeInterpreter => "code",
        ToolCategory::ImageGeneration => "img",
        ToolCategory::Shell => "sh",
        ToolCategory::ApplyPatch => "patch",
        ToolCategory::Namespace => "ns",
        ToolCategory::ToolSearch => "tool_search",
    }
}

fn render_tool_choice_compact(choice: &ToolChoiceParam) -> String {
    match choice {
        ToolChoiceParam::AllowedTools(_) => "tc:allowed_tools".to_string(),
        ToolChoiceParam::Function(tool) => format!("tc:function:{}", tool.name),
        ToolChoiceParam::Mcp(tool) => format!("tc:mcp:{}", tool.name),
        ToolChoiceParam::Custom(tool) => format!("tc:custom:{}", tool.name),
        ToolChoiceParam::ApplyPatch => "tc:apply_patch".to_string(),
        ToolChoiceParam::Shell => "tc:shell".to_string(),
        ToolChoiceParam::Hosted(kind) => format!("tc:{kind}"),
        ToolChoiceParam::Mode(mode) => match mode {
            ToolChoiceOptions::Auto => "tc:auto".to_string(),
            ToolChoiceOptions::None => "tc:none".to_string(),
            ToolChoiceOptions::Required => "tc:required".to_string(),
        },
    }
}

fn render_include_hint(value: &IncludeEnum) -> &'static str {
    match value {
        IncludeEnum::FileSearchCallResults => "file_search_call.results",
        IncludeEnum::WebSearchCallResults => "web_search_call.results",
        IncludeEnum::WebSearchCallActionSources => "web_search_call.action.sources",
        IncludeEnum::MessageInputImageImageUrl => "message.input_image.image_url",
        IncludeEnum::ComputerCallOutputOutputImageUrl => "computer_call_output.output.image_url",
        IncludeEnum::CodeInterpreterCallOutputs => "code_interpreter_call.outputs",
        IncludeEnum::ReasoningEncryptedContent => "reasoning.enc",
        IncludeEnum::MessageOutputTextLogprobs => "message.output_text.logprobs",
    }
}
