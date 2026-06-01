//! OpenAI Chat Completions protocol-native response helpers.
//!
//! Keep protocol-facing response projection types here. Provider-local
//! observation and summary views live under `provider/openai/chat_completions/`.

mod projection;

pub use self::projection::{ChatResponseProjection, ChatStreamResponseProjection};
