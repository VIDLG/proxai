//! Translation entrypoints rooted at the `anthropic_messages` inbound protocol.
//!
//! Only explicit cross-protocol conversions live here.

pub(crate) mod to_openai_chat_completions;
pub(crate) mod to_openai_responses;
