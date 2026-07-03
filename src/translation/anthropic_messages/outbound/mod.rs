//! Outbound Anthropic Messages protocol helpers for translation pairs that
//! target Anthropic Messages (`chat -> anthropic`, `responses -> anthropic`, ...).
//!
//! This module is consumed by other translation pairs whose target protocol
//! is Anthropic Messages. It is NOT part of the inbound `anthropic_messages -> *`
//! flow; that flow owns its own modules (`streaming`, `to_*`).

mod request;
mod response;

pub(crate) use request::*;
pub(crate) use response::*;
