use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::translation::{TranslationError, TranslationResult};

impl From<responses::ServiceTier> for chat::ServiceTier {
    fn from(value: responses::ServiceTier) -> Self {
        match value {
            responses::ServiceTier::Auto => Self::Auto,
            responses::ServiceTier::Default => Self::Default,
            responses::ServiceTier::Flex => Self::Flex,
            responses::ServiceTier::Scale => Self::Scale,
            responses::ServiceTier::Priority => Self::Priority,
        }
    }
}

impl From<&responses::Reasoning> for chat::ReasoningEffort {
    fn from(reasoning: &responses::Reasoning) -> Self {
        reasoning.effort.map(Into::into).unwrap_or_default()
    }
}

impl From<responses::ReasoningEffort> for chat::ReasoningEffort {
    fn from(effort: responses::ReasoningEffort) -> Self {
        match effort {
            responses::ReasoningEffort::None => Self::None,
            responses::ReasoningEffort::Minimal => Self::Minimal,
            responses::ReasoningEffort::Low => Self::Low,
            responses::ReasoningEffort::Medium => Self::Medium,
            responses::ReasoningEffort::High => Self::High,
            responses::ReasoningEffort::Xhigh => Self::Xhigh,
        }
    }
}

impl TryFrom<&responses::TextResponseFormatConfiguration> for chat::ResponseFormat {
    type Error = TranslationError;

    fn try_from(value: &responses::TextResponseFormatConfiguration) -> TranslationResult<Self> {
        match value {
            responses::TextResponseFormatConfiguration::Text => Ok(Self::Text),
            responses::TextResponseFormatConfiguration::JsonObject => Ok(Self::JsonObject),
            responses::TextResponseFormatConfiguration::JsonSchema(schema) => {
                Ok(Self::JsonSchema {
                    json_schema: chat_request::ResponseFormatJsonSchema {
                        description: schema.description.clone(),
                        name: schema.name.clone(),
                        schema: schema.schema.clone(),
                        strict: schema.strict,
                    },
                })
            }
        }
    }
}

impl From<responses::Verbosity> for chat::Verbosity {
    fn from(value: responses::Verbosity) -> Self {
        match value {
            responses::Verbosity::Low => Self::Low,
            responses::Verbosity::Medium => Self::Medium,
            responses::Verbosity::High => Self::High,
        }
    }
}

impl From<&responses::ResponseStreamOptions> for chat::ChatCompletionStreamOptions {
    fn from(value: &responses::ResponseStreamOptions) -> Self {
        if value.include_obfuscation.is_some() {
            tracing::trace!(
                source_field = "stream_options.include_obfuscation",
                reason = "Chat Completions stream_options has no include_obfuscation equivalent",
                "skipping Responses stream field during Chat Completions translation"
            );
        }
        Self {
            include_usage: None,
            include_obfuscation: value.include_obfuscation,
        }
    }
}
