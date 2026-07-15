pub mod anthropic;
mod error;
mod field_presence;
pub mod openai;
pub use error::ErrorObject;
pub use field_presence::{Nullable, OptionalNullable, RequiredNullable, deserialize_present};
pub use openai::responses as openai_responses;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumMessage, EnumString};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Deserialize,
    Serialize,
    Display,
    EnumMessage,
    EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum RequestProtocol {
    #[default]
    #[strum(message = "OpenAI Responses")]
    OpenaiResponses,
    #[strum(message = "OpenAI Chat Completions")]
    OpenaiChatCompletions,
    #[strum(message = "Anthropic Messages")]
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
    pub fn human_name(self) -> &'static str {
        self.get_message()
            .expect("all request protocol variants must define a human-readable name")
    }

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
