use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use super::{ToolChoiceCustom, ToolChoiceFunction, ToolChoiceMCP};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ToolChoiceAllowedMode {
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolChoiceAllowed {
    pub mode: ToolChoiceAllowedMode,
    pub tools: Vec<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ToolChoiceTypes {
    FileSearch,
    WebSearchPreview,
    Computer,
    ComputerUsePreview,
    ComputerUse,
    #[serde(rename = "web_search_preview_2025_03_11")]
    #[strum(to_string = "web_search_preview_2025_03_11")]
    WebSearchPreview20250311,
    CodeInterpreter,
    ImageGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ToolChoiceOptions {
    None,
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
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
