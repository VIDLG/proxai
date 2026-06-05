pub mod anthropic;
mod error;
pub mod openai;
pub use error::ErrorObject;
pub use openai::responses as openai_responses;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum RequestProtocol {
    #[default]
    OpenaiResponses,
    OpenaiChatCompletions,
    AnthropicMessages,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ProviderProtocol {
    #[default]
    OpenaiResponses,
    OpenaiChatCompletions,
    AnthropicMessages,
}

impl RequestProtocol {
    pub fn matches_provider_protocol(self, provider_protocol: ProviderProtocol) -> bool {
        self == provider_protocol.default_request_protocol()
    }
}

impl ProviderProtocol {
    pub fn default_request_protocol(self) -> RequestProtocol {
        match self {
            Self::OpenaiResponses => RequestProtocol::OpenaiResponses,
            Self::OpenaiChatCompletions => RequestProtocol::OpenaiChatCompletions,
            Self::AnthropicMessages => RequestProtocol::AnthropicMessages,
        }
    }
}
