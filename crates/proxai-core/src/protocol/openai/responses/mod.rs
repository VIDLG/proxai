//! OpenAI Responses protocol-native helpers and shared schema behavior.
//!
//! This namespace is the home for same-protocol helpers that do not belong in
//! `translation/` or `provider/`, such as protocol-native request and response
//! projections and reusable Responses-specific logic.

pub mod request;
pub mod response;
pub mod wire;

#[allow(unused_imports, reason = "OpenAI Responses facade re-exports.")]
pub use self::request::*;
#[allow(unused_imports, reason = "OpenAI Responses facade re-exports.")]
pub use self::response::*;
#[allow(unused_imports, reason = "OpenAI Responses facade re-exports.")]
pub use self::wire::*;
