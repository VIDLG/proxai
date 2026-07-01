use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai_responses as responses;
use crate::translation::TranslationResult;

impl From<chat::ServiceTier> for responses::ServiceTier {
    fn from(value: chat::ServiceTier) -> Self {
        match value {
            chat::ServiceTier::Auto => responses::ServiceTier::Auto,
            chat::ServiceTier::Default => responses::ServiceTier::Default,
            chat::ServiceTier::Flex => responses::ServiceTier::Flex,
            chat::ServiceTier::Scale => responses::ServiceTier::Scale,
            chat::ServiceTier::Priority => responses::ServiceTier::Priority,
        }
    }
}

impl From<chat::ReasoningEffort> for responses::Reasoning {
    fn from(effort: chat::ReasoningEffort) -> Self {
        Self {
            effort: Some(match effort {
                chat::ReasoningEffort::None => responses::ReasoningEffort::None,
                chat::ReasoningEffort::Minimal => responses::ReasoningEffort::Minimal,
                chat::ReasoningEffort::Low => responses::ReasoningEffort::Low,
                chat::ReasoningEffort::Medium => responses::ReasoningEffort::Medium,
                chat::ReasoningEffort::High => responses::ReasoningEffort::High,
                chat::ReasoningEffort::Xhigh => responses::ReasoningEffort::Xhigh,
            }),
            summary: None,
        }
    }
}

impl TryFrom<&chat::ResponseFormat> for responses::TextResponseFormatConfiguration {
    type Error = crate::translation::TranslationError;

    fn try_from(value: &chat::ResponseFormat) -> TranslationResult<Self> {
        match value {
            chat::ResponseFormat::Text => Ok(Self::Text),
            chat::ResponseFormat::JsonObject => Ok(Self::JsonObject),
            chat::ResponseFormat::JsonSchema { json_schema } => {
                Ok(Self::JsonSchema(responses::ResponseFormatJsonSchema {
                    description: json_schema.description.clone(),
                    name: json_schema.name.clone(),
                    schema: json_schema.schema.clone(),
                    strict: json_schema.strict,
                }))
            }
        }
    }
}

impl From<chat::Verbosity> for responses::Verbosity {
    fn from(value: chat::Verbosity) -> Self {
        match value {
            chat::Verbosity::Low => Self::Low,
            chat::Verbosity::Medium => Self::Medium,
            chat::Verbosity::High => Self::High,
        }
    }
}

impl From<chat::ChatCompletionStreamOptions> for responses::ResponseStreamOptions {
    fn from(value: chat::ChatCompletionStreamOptions) -> Self {
        if value.include_usage.is_some() {
            tracing::trace!(
                source_field = "stream_options.include_usage",
                reason = "OpenAI Responses stream_options schema has no include_usage field",
                "skipping Chat Completions request field during OpenAI Responses translation"
            );
        }

        Self {
            include_obfuscation: value.include_obfuscation,
        }
    }
}
