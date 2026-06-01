//! OpenAI Chat Completions request protocol projection facade.

mod projection;
pub mod wire;

pub use projection::RequestProjection;
pub use wire::{
    ChatCompletionAudio, ChatCompletionStreamOptions, ChatCompletionToolChoiceOption,
    ChatCompletionTools, PredictionContent, ReasoningEffort, ResponseFormat, ResponseModalities,
    StopConfiguration, ToolChoiceOptions, Verbosity, WebSearchOptions,
};
