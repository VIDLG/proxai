//! Request-only input field conversions for `openai_responses -> openai_chat_completions`.
//!
//! Only `From` / `TryFrom` impls consumed by the pair's `request` child belong
//! here. Conversions shared across `request` / `response` / `streaming` children
//! (usage, tool-call structs, service tier) live in pair-root `types.rs`.

use crate::protocol::openai::chat_completions as chat;
use crate::protocol::openai::chat_completions::request::wire as chat_request;
use crate::protocol::openai::responses;
use crate::protocol::openai::{PromptCacheBreakpointConfig, PromptCacheBreakpointParam};
use crate::translation::{TranslationError, TranslationResult};

impl From<&PromptCacheBreakpointConfig> for PromptCacheBreakpointParam {
    fn from(value: &PromptCacheBreakpointConfig) -> Self {
        Self { mode: value.mode }
    }
}

impl From<&responses::InputTextContent>
    for chat_request::ChatCompletionRequestMessageContentPartText
{
    fn from(value: &responses::InputTextContent) -> Self {
        Self {
            text: value.text.clone(),
            prompt_cache_breakpoint: value.prompt_cache_breakpoint.as_ref().map(Into::into),
        }
    }
}

impl From<responses::ImageDetail> for chat::ImageDetail {
    fn from(value: responses::ImageDetail) -> Self {
        match value {
            responses::ImageDetail::Auto | responses::ImageDetail::Original => Self::Auto,
            responses::ImageDetail::Low => Self::Low,
            responses::ImageDetail::High => Self::High,
        }
    }
}

impl From<responses::PromptCacheRetention> for chat::PromptCacheRetention {
    fn from(value: responses::PromptCacheRetention) -> Self {
        match value {
            responses::PromptCacheRetention::InMemory => Self::InMemory,
            responses::PromptCacheRetention::Hours24 => Self::Hours24,
        }
    }
}

impl From<&responses::Reasoning> for chat::ReasoningEffort {
    fn from(reasoning: &responses::Reasoning) -> Self {
        reasoning
            .effort
            .as_non_null()
            .copied()
            .map(Into::into)
            .unwrap_or_default()
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
            responses::ReasoningEffort::Max => Self::Max,
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
                        strict: schema.strict.as_non_null().copied(),
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
        Self {
            include_usage: None,
            include_obfuscation: value.include_obfuscation,
        }
    }
}

impl From<responses::CustomToolParamFormat> for chat_request::CustomToolPropertiesFormat {
    fn from(value: responses::CustomToolParamFormat) -> Self {
        match value {
            responses::CustomToolParamFormat::Text => Self::Text,
            responses::CustomToolParamFormat::Grammar(grammar) => {
                Self::Grammar(chat_request::CustomGrammarFormatParam {
                    definition: grammar.definition,
                    syntax: grammar.syntax.into(),
                })
            }
        }
    }
}

impl From<responses::GrammarSyntax> for chat_request::GrammarSyntax {
    fn from(value: responses::GrammarSyntax) -> Self {
        match value {
            responses::GrammarSyntax::Lark => Self::Lark,
            responses::GrammarSyntax::Regex => Self::Regex,
        }
    }
}

impl From<responses::ToolChoiceAllowedMode> for chat_request::ToolChoiceAllowedMode {
    fn from(value: responses::ToolChoiceAllowedMode) -> Self {
        match value {
            responses::ToolChoiceAllowedMode::Auto => Self::Auto,
            responses::ToolChoiceAllowedMode::Required => Self::Required,
        }
    }
}
