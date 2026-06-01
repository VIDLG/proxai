use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

use super::{ToolChoiceCustom, ToolChoiceFunction, ToolChoiceMCP};

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ToolChoiceAllowedMode))]
#[strum(serialize_all = "snake_case")]
pub enum ToolChoiceAllowedMode {
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolChoiceAllowed))]
pub struct ToolChoiceAllowed {
    pub mode: ToolChoiceAllowedMode,
    pub tools: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ToolChoiceTypes))]
#[strum(serialize_all = "snake_case")]
pub enum ToolChoiceTypes {
    FileSearch,
    WebSearchPreview,
    Computer,
    ComputerUsePreview,
    ComputerUse,
    #[strum(to_string = "web_search_preview_2025_03_11")]
    WebSearchPreview20250311,
    CodeInterpreter,
    ImageGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ToolChoiceOptions))]
#[strum(serialize_all = "snake_case")]
pub enum ToolChoiceOptions {
    None,
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ToolChoiceParam))]
pub enum ToolChoiceParam {
    AllowedTools(ToolChoiceAllowed),
    Function(ToolChoiceFunction),
    Mcp(ToolChoiceMCP),
    Custom(ToolChoiceCustom),
    ApplyPatch,
    Shell,
    Hosted(ToolChoiceTypes),
    Mode(ToolChoiceOptions),
}
