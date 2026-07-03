//! Translation entrypoints rooted at the `openai_responses` inbound protocol.
//!
//! Only explicit cross-protocol conversions live here, plus the shared inbound
//! streaming lifecycle wrapper used by every `responses -> *` translator.

pub(crate) mod outbound;
pub(crate) mod streaming;
pub(crate) mod to_anthropic_messages;
pub(crate) mod to_openai_chat_completions;
