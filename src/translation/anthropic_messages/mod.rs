//! Translation entrypoints rooted at the `anthropic_messages` inbound protocol.
//!
//! Only explicit cross-protocol conversions live here. The shared inbound
//! streaming lifecycle ([`streaming`]) is owned by this module because
//! every `anthropic_messages -> *` pair enforces the same Anthropic stream
//! ordering rules regardless of the target protocol.

pub(crate) mod streaming;
pub(crate) mod to_openai_chat_completions;
pub(crate) mod to_openai_responses;
