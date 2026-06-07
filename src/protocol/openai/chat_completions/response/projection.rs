use serde::{Deserialize, Serialize};

use super::super::{ChatChoice, ChatChoiceStream, CompletionUsage, ServiceTier};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatResponseProjection {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamResponseProjection {
    pub id: String,
    pub choices: Vec<ChatChoiceStream>,
    pub created: u32,
    pub model: String,
    pub service_tier: Option<ServiceTier>,
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

impl From<super::super::CreateChatCompletionResponse> for ChatResponseProjection {
    fn from(response: super::super::CreateChatCompletionResponse) -> Self {
        Self {
            id: response.id,
            choices: response.choices,
            created: response.created,
            model: response.model,
            service_tier: response.service_tier,
            object: response.object,
            usage: response.usage,
        }
    }
}

impl From<super::super::CreateChatCompletionStreamResponse> for ChatStreamResponseProjection {
    fn from(response: super::super::CreateChatCompletionStreamResponse) -> Self {
        Self {
            id: response.id,
            choices: response.choices,
            created: response.created,
            model: response.model,
            service_tier: response.service_tier,
            object: response.object,
            usage: response.usage,
        }
    }
}
