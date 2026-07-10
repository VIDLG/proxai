use serde::{Deserialize, Serialize};
use strum::Display;

use super::{CompletionUsage, ServiceTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Assistant,
    Tool,
    Function,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTool {
    pub name: String,
    pub input: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageCustomToolCall {
    pub id: String,
    pub custom_tool: CustomTool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionMessageToolCalls {
    Function(ChatCompletionMessageToolCall),
    Custom(ChatCompletionMessageCustomToolCall),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessageAudio {
    pub id: String,
    pub expires_at: u64,
    pub data: String,
    pub transcript: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlCitation {
    pub end_index: u32,
    pub start_index: u32,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatCompletionResponseMessageAnnotation {
    UrlCitation { url_citation: UrlCitation },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopLogprobs {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompletionTokenLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<TopLogprobs>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoiceLogprobs {
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionResponseMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// OpenAI-compatible reasoning extension consumed by Zed as thinking output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCalls>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<ChatCompletionResponseMessageAnnotation>>,
    pub role: Role,
    // Deprecated function_call is intentionally not projected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<ChatCompletionResponseMessageAudio>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatCompletionResponseMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateChatCompletionResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,
    // Deprecated system_fingerprint is intentionally not projected.
    pub object: String,
    pub usage: Option<CompletionUsage>,
}
