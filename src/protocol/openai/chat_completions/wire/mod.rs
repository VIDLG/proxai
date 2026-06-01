//! OpenAI Chat Completions protocol-native data types.

mod common;
mod messages;
mod response;
mod stream;
mod usage;

#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::common::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::messages::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::response::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::stream::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::usage::*;
