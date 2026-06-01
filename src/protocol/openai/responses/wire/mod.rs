//! OpenAI Responses protocol-native data types.

mod annotation;
mod common;
mod compaction;
mod include;
mod input_content;
mod input_item;
mod message;
mod output_item;
mod prompt;
mod reasoning;
pub mod response;
mod shared;
mod stream;
mod tool_choice;
mod tools;

#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::annotation::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::common::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::compaction::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::include::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::input_content::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::input_item::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::message::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::output_item::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::prompt::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::reasoning::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::response::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::shared::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::stream::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::tool_choice::*;
#[allow(
    unused_imports,
    reason = "Facade re-exports for request and response protocol modules."
)]
pub use self::tools::*;
// Re-export for compare script scanning
pub use crate::protocol::ErrorObject;
