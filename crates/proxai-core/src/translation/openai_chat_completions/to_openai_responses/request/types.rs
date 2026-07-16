use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai_responses as responses;
use crate::translation::{TranslationError, TranslationResult, TranslationScope};

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

impl From<chat::PromptCacheRetention> for responses::PromptCacheRetention {
    fn from(value: chat::PromptCacheRetention) -> Self {
        match value {
            chat::PromptCacheRetention::InMemory => Self::InMemory,
            chat::PromptCacheRetention::Hours24 => Self::Hours24,
        }
    }
}

impl From<chat::ReasoningEffort> for responses::Reasoning {
    fn from(effort: chat::ReasoningEffort) -> Self {
        Self {
            mode: None,
            effort: Some(match effort {
                chat::ReasoningEffort::None => responses::ReasoningEffort::None,
                chat::ReasoningEffort::Minimal => responses::ReasoningEffort::Minimal,
                chat::ReasoningEffort::Low => responses::ReasoningEffort::Low,
                chat::ReasoningEffort::Medium => responses::ReasoningEffort::Medium,
                chat::ReasoningEffort::High => responses::ReasoningEffort::High,
                chat::ReasoningEffort::Xhigh => responses::ReasoningEffort::Xhigh,
                chat::ReasoningEffort::Max => responses::ReasoningEffort::Max,
            })
            .into(),
            summary: None.into(),
            context: None.into(),
            generate_summary: None.into(),
        }
    }
}

impl TryFrom<&chat::ResponseFormat> for responses::TextResponseFormatConfiguration {
    type Error = TranslationError;

    fn try_from(value: &chat::ResponseFormat) -> TranslationResult<Self> {
        match value {
            chat::ResponseFormat::Text => Ok(Self::Text),
            chat::ResponseFormat::JsonObject => Ok(Self::JsonObject),
            chat::ResponseFormat::JsonSchema { json_schema } => {
                Ok(Self::JsonSchema(responses::TextResponseFormatJsonSchema {
                    description: json_schema.description.clone(),
                    name: json_schema.name.clone(),
                    schema: json_schema.schema.clone(),
                    strict: json_schema.strict.into(),
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

pub(super) fn response_stream_options(
    value: &chat::ChatCompletionStreamOptions,
    scope: &TranslationScope,
) -> responses::ResponseStreamOptions {
    if value.include_usage.is_some() {
        scope.dropped(
            "Chat stream_options.include_usage",
            "OpenAI Responses stream_options has no include_usage field",
        );
    }

    responses::ResponseStreamOptions {
        include_obfuscation: value.include_obfuscation,
    }
}
