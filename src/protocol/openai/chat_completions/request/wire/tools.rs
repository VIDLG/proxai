use async_openai::types::chat as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;

// Tool definitions.

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionObject))]
pub struct FunctionObject {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<Value>,
    pub strict: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionTool))]
pub struct ChatCompletionTool {
    pub function: FunctionObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::GrammarSyntax))]
pub enum GrammarSyntax {
    Lark,
    #[default]
    Regex,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomGrammarFormatParam))]
pub struct CustomGrammarFormatParam {
    pub definition: String,
    pub syntax: GrammarSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolPropertiesFormat))]
pub enum CustomToolPropertiesFormat {
    #[default]
    Text,
    Grammar {
        grammar: CustomGrammarFormatParam,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolProperties))]
pub struct CustomToolProperties {
    pub name: String,
    pub description: Option<String>,
    pub format: CustomToolPropertiesFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomToolChatCompletions))]
pub struct CustomToolChatCompletions {
    pub custom: CustomToolProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionTools))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionTools {
    Function(ChatCompletionTool),
    Custom(CustomToolChatCompletions),
}

// Direct named tool choices.

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionName))]
pub struct FunctionName {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionNamedToolChoice))]
pub struct ChatCompletionNamedToolChoice {
    pub function: FunctionName,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CustomName))]
pub struct CustomName {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionNamedToolChoiceCustom))]
pub struct ChatCompletionNamedToolChoiceCustom {
    pub custom: CustomName,
}

// Allowed-tools choices.

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ToolChoiceAllowedMode))]
pub enum ToolChoiceAllowedMode {
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionAllowedTools))]
pub struct ChatCompletionAllowedTools {
    pub mode: ToolChoiceAllowedMode,
    pub tools: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionAllowedToolsChoice))]
pub struct ChatCompletionAllowedToolsChoice {
    pub allowed_tools: Vec<ChatCompletionAllowedTools>,
}

// Tool choice union.

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ToolChoiceOptions))]
pub enum ToolChoiceOptions {
    None,
    Auto,
    Required,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionToolChoiceOption))]
#[serde(untagged)]
pub enum ChatCompletionToolChoiceOption {
    AllowedTools(ChatCompletionAllowedToolsChoice),
    Function(ChatCompletionNamedToolChoice),
    Custom(ChatCompletionNamedToolChoiceCustom),
    Mode(ToolChoiceOptions),
}
