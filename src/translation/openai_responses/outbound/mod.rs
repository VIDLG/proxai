//! Outbound OpenAI Responses protocol helpers for translation pairs that
//! target OpenAI Responses (`anthropic -> responses`, `chat -> responses`, ...).

mod request;
mod response;
mod streaming;

pub(crate) use request::*;
pub(crate) use response::*;
pub(crate) use streaming::*;
