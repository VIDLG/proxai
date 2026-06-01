use structural_convert::StructuralConvert;

use super::super::{
    ChatChoice, ChatChoiceStream, CompletionUsage, CreateChatCompletionResponse,
    CreateChatCompletionStreamResponse, ServiceTier,
};

#[derive(Debug, Clone, Default, PartialEq, StructuralConvert)]
#[convert(from(CreateChatCompletionResponse))]
pub struct ChatResponseProjection {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, StructuralConvert)]
#[convert(from(CreateChatCompletionStreamResponse))]
pub struct ChatStreamResponseProjection {
    pub id: String,
    pub choices: Vec<ChatChoiceStream>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}
