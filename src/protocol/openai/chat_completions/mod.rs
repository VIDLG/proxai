//! OpenAI Chat Completions protocol-native helpers and schema behavior.

pub mod request;
pub mod response;
pub mod wire;

#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::request::*;
#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::response::*;
#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::wire::*;
