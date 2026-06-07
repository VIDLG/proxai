use serde::{Deserialize, Serialize};
use serde_json::Value;

// Tool definitions.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionObject {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionTool {
    pub function: FunctionObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GrammarSyntax {
    Lark,
    #[default]
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CustomGrammarFormatParam {
    pub definition: String,
    pub syntax: GrammarSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CustomToolPropertiesFormat {
    #[default]
    Text,
    Grammar {
        grammar: CustomGrammarFormatParam,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomToolProperties {
    pub name: String,
    pub description: Option<String>,
    pub format: CustomToolPropertiesFormat,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionName {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoice {
    pub function: FunctionName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomName {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionNamedToolChoiceCustom {
    pub custom: CustomName,
}

// Allowed-tools choices.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolChoiceAllowedMode {
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedTools {
    pub mode: ToolChoiceAllowedMode,
    pub tools: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAllowedToolsChoice {
    pub allowed_tools: Vec<ChatCompletionAllowedTools>,
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
#[serde(untagged)]
pub enum ChatCompletionToolChoiceOption {
    AllowedTools(ChatCompletionAllowedToolsChoice),
    Function(ChatCompletionNamedToolChoice),
    Custom(ChatCompletionNamedToolChoiceCustom),
    Mode(ToolChoiceOptions),
}
