//! OpenAI Chat Completions request protocol projection facade.

mod projection;
pub(crate) mod wire;

pub use projection::RequestProjection;
pub use wire::{
    ChatCompletionAudio, ChatCompletionFunctionCall, ChatCompletionFunctions,
    ChatCompletionStreamOptions, ChatCompletionToolChoiceOption, ChatCompletionTools,
    PredictionContent, PromptCacheRetention, ReasoningEffort, ResponseFormat, ResponseModalities,
    StopConfiguration, ToolChoiceOptions, Verbosity, WebSearchOptions,
};
