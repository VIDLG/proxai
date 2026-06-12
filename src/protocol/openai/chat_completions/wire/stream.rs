use serde::{Deserialize, Serialize};

use super::{ChatChoiceLogprobs, CompletionUsage, FinishReason, Role, ServiceTier};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCallStream {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionType {
    Function,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionMessageToolCallChunk {
    pub index: u32,
    pub id: Option<String>,
    pub r#type: Option<FunctionType>,
    pub function: Option<FunctionCallStream>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionStreamResponseDelta {
    pub content: Option<String>,
    // Deprecated function_call is intentionally not projected.
    pub tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
    pub role: Option<Role>,
    pub refusal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatChoiceStream {
    pub index: u32,
    pub delta: ChatCompletionStreamResponseDelta,
    pub finish_reason: Option<FinishReason>,
    pub logprobs: Option<ChatChoiceLogprobs>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
