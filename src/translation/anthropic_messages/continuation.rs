//! Client-carried Anthropic continuation data.
//!
//! Anthropic thinking signatures and redacted-thinking payloads cannot be
//! represented faithfully by every OpenAI-compatible protocol. For clients
//! that replay their assistant history, ProxAI carries that provider-scoped
//! state in a versioned envelope and removes it before sending a later request
//! back to Anthropic.

use delegate::delegate;
use derive_more::From;
use serde::{Deserialize, Serialize};

pub(crate) const ENVELOPE_PREFIX: &str = "proxai:anthropic:v1:";
const CHAT_ENVELOPE_SEPARATOR: &str = "\n\n";
const MAX_ENVELOPE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Continuation {
    Thinking { thinking: String, signature: String },
    RedactedThinking { data: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, From, Serialize, Deserialize)]
pub(crate) struct ContinuationEnvelope {
    blocks: Vec<Continuation>,
}

impl IntoIterator for ContinuationEnvelope {
    type Item = Continuation;
    type IntoIter = std::vec::IntoIter<Continuation>;

    fn into_iter(self) -> Self::IntoIter {
        self.blocks.into_iter()
    }
}

impl ContinuationEnvelope {
    delegate! {
        to self.blocks {
            pub(crate) fn push(&mut self, continuation: Continuation);
            pub(crate) fn is_empty(&self) -> bool;
        }
    }

    pub(crate) fn encode(self) -> serde_json::Result<String> {
        let envelope = format!("{ENVELOPE_PREFIX}{}", serde_json::to_string(&self)?);
        if envelope.len() > MAX_ENVELOPE_BYTES {
            return Err(serde_json::Error::io(std::io::Error::other(
                "Anthropic continuation envelope exceeds the 256 KiB limit",
            )));
        }
        Ok(envelope)
    }

    pub(crate) fn decode(value: &str) -> serde_json::Result<Option<Self>> {
        let Some(payload) = value.strip_prefix(ENVELOPE_PREFIX) else {
            return Ok(None);
        };
        if value.len() > MAX_ENVELOPE_BYTES {
            return Err(serde_json::Error::io(std::io::Error::other(
                "Anthropic continuation envelope exceeds the 256 KiB limit",
            )));
        }
        serde_json::from_str(payload).map(Some)
    }
    pub(crate) fn append_to_chat_reasoning_content(
        self,
        visible_reasoning: String,
    ) -> serde_json::Result<String> {
        let envelope = self.encode()?;
        Ok(if visible_reasoning.is_empty() {
            envelope
        } else {
            format!("{visible_reasoning}{CHAT_ENVELOPE_SEPARATOR}{envelope}")
        })
    }

    pub(crate) fn split_chat_reasoning_content(
        value: &str,
    ) -> serde_json::Result<(String, Option<Self>)> {
        if let Some(envelope) = Self::decode(value)? {
            return Ok((String::new(), Some(envelope)));
        }

        let Some((visible_reasoning, envelope)) = value.rsplit_once(CHAT_ENVELOPE_SEPARATOR) else {
            return Ok((value.to_string(), None));
        };
        Ok((visible_reasoning.to_string(), Self::decode(envelope)?))
    }
}

#[cfg(test)]
#[path = "continuation_tests.rs"]
mod tests;
