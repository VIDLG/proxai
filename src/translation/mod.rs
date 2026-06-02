//! Protocol translation layer.
//!
//! This layer is responsible for explicit cross-protocol conversions between the
//! inbound request protocol and the selected provider protocol.
//!
//! Keep translation concerns here rather than inside `provider/` so that:
//! - `ingress/` owns inbound protocol parsing and normalization
//! - `translation/` owns protocol-to-protocol conversion
//! - `provider/` owns transport and provider-local request/response rendering
//!
//! Pair modules are organized by inbound protocol root and explicit `to_*`
//! targets so that protocol translation stays visible and easy to audit.
//! Self-to-self protocol paths are intentionally omitted.

pub(crate) mod anthropic_messages;
pub(crate) mod openai_chat_completions;
pub(crate) mod openai_responses;
mod request;
mod response;
mod sse;

pub(crate) use request::translate_request;
pub(crate) use response::translate_response;
