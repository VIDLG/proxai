//! Translation entrypoints rooted at the `openai_responses` inbound protocol.
//!
//! Only explicit cross-protocol conversions live here.

pub(crate) mod to_anthropic_messages;
pub(crate) mod to_openai_chat_completions;
