//! Outbound OpenAI Responses protocol helpers for translation pairs that
//! target OpenAI Responses (`anthropic -> responses`, `chat -> responses`, ...).
//!
//! This module is consumed by other translation pairs whose target protocol
//! is OpenAI Responses. It is NOT part of the inbound `openai_responses -> *`
//! flow; that flow owns its own modules (`streaming`, `to_*`).

mod request;
mod response;

pub(crate) use request::*;
pub(crate) use response::*;
