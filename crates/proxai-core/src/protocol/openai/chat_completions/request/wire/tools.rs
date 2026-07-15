use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// OpenAPI schema: `#/components/schemas/ChatCompletionFunctionCallOption`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionFunctionCallOption {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyFunctionCallMode {
    None,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatCompletionFunctionCall {
    Mode(LegacyFunctionCallMode),
    Named(ChatCompletionFunctionCallOption),
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionFunctions`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionFunctions {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub parameters: Option<Value>,
}

// Tool definitions.

/// OpenAPI schema: `#/components/schemas/FunctionObject`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionObject {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,

    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub parameters: Option<Value>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub strict: OptionalNullable<bool>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionTool {
    pub function: FunctionObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrammarSyntax {
    Lark,
    #[default]
    Regex,
}

/// OpenAPI schema: `#/components/schemas/CustomGrammarFormatParam`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CustomGrammarFormatParam {
    pub syntax: GrammarSyntax,

    pub definition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CustomToolPropertiesFormat {
    #[default]
    Text,
    Grammar(CustomGrammarFormatParam),
}

/// OpenAPI schema:
/// `#/components/schemas/CustomToolChatCompletions/properties/custom`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolProperties {
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub format: Option<CustomToolPropertiesFormat>,
}

/// OpenAPI schema: `#/components/schemas/CustomToolChatCompletions`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolChatCompletions {
    pub custom: CustomToolProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionTools {
    Function(ChatCompletionTool),
    Custom(CustomToolChatCompletions),
}

// Direct named tool choices.

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionNamedToolChoice/properties/function`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionName {
    pub name: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionNamedToolChoice`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoice {
    pub function: FunctionName,
}

/// OpenAPI schema:
/// `#/components/schemas/ChatCompletionNamedToolChoiceCustom/properties/custom`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomName {
    pub name: String,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionNamedToolChoiceCustom`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoiceCustom {
    pub custom: CustomName,
}

// Allowed-tools choices.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceAllowedMode {
    Auto,
    Required,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionAllowedTools`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedTools {
    pub mode: ToolChoiceAllowedMode,
    pub tools: Vec<Value>,
}

/// OpenAPI schema: `#/components/schemas/ChatCompletionAllowedToolsChoice`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedToolsChoice {
    pub allowed_tools: ChatCompletionAllowedTools,
}

// Tool choice union.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoiceOptions {
    None,
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionToolChoiceOption {
    AllowedTools(ChatCompletionAllowedToolsChoice),
    Function(ChatCompletionNamedToolChoice),
    Custom(ChatCompletionNamedToolChoiceCustom),
    #[serde(untagged)]
    Mode(ToolChoiceOptions),
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
