//! OpenAI Chat Completions protocol-native helpers and schema behavior.

pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod wire;

#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::request::*;
#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::response::*;
#[allow(unused_imports, reason = "OpenAI Chat Completions facade re-exports.")]
pub use self::wire::*;
