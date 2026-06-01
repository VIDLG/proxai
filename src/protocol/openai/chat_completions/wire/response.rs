use async_openai::types::chat as openai;
use structural_convert::StructuralConvert;
use strum::Display;

use super::{CompletionUsage, ServiceTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::FinishReason))]
#[strum(serialize_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    FunctionCall,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::Role))]
#[strum(serialize_all = "lowercase")]
pub enum Role {
    System,
    #[default]
    User,
    Assistant,
    Tool,
    Function,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::FunctionCall))]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionMessageToolCall))]
pub struct ChatCompletionMessageToolCall {
    pub id: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::CustomTool))]
pub struct CustomTool {
    pub name: String,
    pub input: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionMessageCustomToolCall))]
pub struct ChatCompletionMessageCustomToolCall {
    pub id: String,
    pub custom_tool: CustomTool,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionMessageToolCalls))]
pub enum ChatCompletionMessageToolCalls {
    Function(ChatCompletionMessageToolCall),
    Custom(ChatCompletionMessageCustomToolCall),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionResponseMessageAudio))]
pub struct ChatCompletionResponseMessageAudio {
    pub id: String,
    pub expires_at: u64,
    pub data: String,
    pub transcript: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::UrlCitation))]
pub struct UrlCitation {
    pub end_index: u32,
    pub start_index: u32,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionResponseMessageAnnotation))]
pub enum ChatCompletionResponseMessageAnnotation {
    UrlCitation { url_citation: UrlCitation },
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::TopLogprobs))]
pub struct TopLogprobs {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ChatCompletionTokenLogprob))]
pub struct ChatCompletionTokenLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Vec<TopLogprobs>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ChatChoiceLogprobs))]
pub struct ChatChoiceLogprobs {
    pub content: Option<Vec<ChatCompletionTokenLogprob>>,
    pub refusal: Option<Vec<ChatCompletionTokenLogprob>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionResponseMessage))]
pub struct ChatCompletionResponseMessage {
    pub content: Option<String>,
    pub refusal: Option<String>,
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCalls>>,
    pub annotations: Option<Vec<ChatCompletionResponseMessageAnnotation>>,
    pub role: Role,
    // Deprecated function_call is intentionally not projected.
    pub audio: Option<ChatCompletionResponseMessageAudio>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ChatChoice))]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatCompletionResponseMessage,
    pub finish_reason: Option<FinishReason>,
    pub logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Clone, Default, PartialEq, StructuralConvert)]
#[convert(from(openai::CreateChatCompletionResponse))]
pub struct CreateChatCompletionResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    // Deprecated system_fingerprint is intentionally not projected.
    pub object: String,
    pub usage: Option<CompletionUsage>,
}
