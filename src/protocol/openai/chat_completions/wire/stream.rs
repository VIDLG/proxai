use async_openai::types::chat as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;

use super::{ChatChoiceLogprobs, CompletionUsage, FinishReason, Role, ServiceTier};

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionCallStream))]
pub struct FunctionCallStream {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::FunctionType))]
pub enum FunctionType {
    Function,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionMessageToolCallChunk))]
pub struct ChatCompletionMessageToolCallChunk {
    pub index: u32,
    pub id: Option<String>,
    pub r#type: Option<FunctionType>,
    pub function: Option<FunctionCallStream>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionStreamResponseDelta))]
pub struct ChatCompletionStreamResponseDelta {
    pub content: Option<String>,
    // Deprecated function_call is intentionally not projected.
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
    pub role: Option<Role>,
    pub refusal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatChoiceStream))]
pub struct ChatChoiceStream {
    pub index: u32,
    pub delta: ChatCompletionStreamResponseDelta,
    pub finish_reason: Option<FinishReason>,
    pub logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Clone, Default, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CreateChatCompletionStreamResponse))]
pub struct CreateChatCompletionStreamResponse {
    pub id: String,
    pub choices: Vec<ChatChoiceStream>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    // Deprecated system_fingerprint is intentionally not projected.
    pub object: String,
    pub usage: Option<CompletionUsage>,
}
