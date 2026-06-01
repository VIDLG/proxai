//! OpenAI Responses protocol-native response helpers.
//!
//! Keep protocol-facing response schema and projection types here. Provider-local
//! observation and summary views live under `provider/openai/responses/`.

mod projection;

pub use self::projection::ResponseProjection;
