#![allow(
    ambiguous_glob_reexports,
    reason = "Anthropic facade re-exports generated helper names that overlap across request and tool schemas."
)]

pub mod blocks;
pub mod citations;
pub mod common;
pub mod content;
pub mod message;
pub mod request;
pub mod stream;
pub mod tools;

// Re-export everything from submodules at `pub` visibility so the facade is fully
// accessible from `messages/mod.rs` and from integration tests.
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::blocks::*;
pub use self::citations::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::common::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::content::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::message::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::request::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::stream::*;
#[allow(
    unused_imports,
    reason = "Anthropic Messages protocol facade re-exports."
)]
pub use self::tools::*;
