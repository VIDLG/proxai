//! OpenAI-compatible Chat extensions understood by Zed.
//!
//! These fields are intentionally kept out of `protocol::openai`: they are not
//! part of the official OpenAI OpenAPI schema. Zed emits `reasoning_content` in
//! assistant history and accepts both `reasoning` and `reasoning_content` in
//! streaming deltas.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::translation::{TranslationError, TranslationResult};

const REASONING: &str = "reasoning";
const REASONING_CONTENT: &str = "reasoning_content";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChatRequestExtensions {
    assistant_reasoning: BTreeMap<usize, String>,
}

impl ChatRequestExtensions {
    pub(crate) fn extract(payload: &Value) -> TranslationResult<Self> {
        let Some(messages) = payload.get("messages").and_then(Value::as_array) else {
            return Ok(Self::default());
        };

        let mut assistant_reasoning = BTreeMap::new();
        for (index, message) in messages.iter().enumerate() {
            let Some(object) = message.as_object() else {
                continue;
            };
            let Some(reasoning) = object.get(REASONING_CONTENT) else {
                continue;
            };
            if object.get("role").and_then(Value::as_str) != Some("assistant") {
                return Err(TranslationError::InvalidPayload(format!(
                    "Chat message {index} uses `{REASONING_CONTENT}` outside an assistant message"
                )));
            }
            match reasoning {
                Value::Null => {}
                Value::String(reasoning) => {
                    assistant_reasoning.insert(index, reasoning.clone());
                }
                _ => {
                    return Err(TranslationError::InvalidPayload(format!(
                        "Chat assistant message {index} `{REASONING_CONTENT}` must be a string or null"
                    )));
                }
            }
        }
        Ok(Self {
            assistant_reasoning,
        })
    }

    pub(crate) fn reasoning(&self, message_index: usize) -> Option<&str> {
        self.assistant_reasoning
            .get(&message_index)
            .map(String::as_str)
    }

    pub(crate) fn insert(&mut self, message_index: usize, reasoning: String) {
        if !reasoning.is_empty() {
            self.assistant_reasoning.insert(message_index, reasoning);
        }
    }

    pub(crate) fn append(&mut self, message_index: usize, reasoning: &str) {
        if !reasoning.is_empty() {
            self.assistant_reasoning
                .entry(message_index)
                .or_default()
                .push_str(reasoning);
        }
    }

    pub(crate) fn apply(self, payload: &mut Value) -> TranslationResult<()> {
        let messages = payload
            .get_mut("messages")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| {
                TranslationError::InvalidPayload(
                    "generated Chat request is missing its messages array".to_string(),
                )
            })?;
        for (index, reasoning) in self.assistant_reasoning {
            let message = messages
                .get_mut(index)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    TranslationError::InvalidPayload(format!(
                        "generated Chat request is missing message {index}"
                    ))
                })?;
            message.insert(REASONING_CONTENT.to_string(), Value::String(reasoning));
        }
        Ok(())
    }
}

pub(crate) fn inject_response_reasoning(
    payload: &mut Value,
    reasoning: Option<String>,
) -> TranslationResult<()> {
    let Some(reasoning) = reasoning.filter(|reasoning| !reasoning.is_empty()) else {
        return Ok(());
    };
    let message = payload
        .pointer_mut("/choices/0/message")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            TranslationError::InvalidPayload(
                "generated Chat response is missing choices[0].message".to_string(),
            )
        })?;
    message.insert(REASONING_CONTENT.to_string(), Value::String(reasoning));
    Ok(())
}

pub(crate) fn inject_stream_reasoning(
    payload: &mut Value,
    reasoning: String,
) -> TranslationResult<()> {
    let delta = payload
        .pointer_mut("/choices/0/delta")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            TranslationError::InvalidPayload(
                "generated Chat stream chunk is missing choices[0].delta".to_string(),
            )
        })?;
    delta.insert(REASONING_CONTENT.to_string(), Value::String(reasoning));
    Ok(())
}

pub(crate) fn response_reasoning(payload: &Value) -> Result<Option<String>, String> {
    let Some(message) = payload
        .pointer("/choices/0/message")
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };

    // Zed deserializes a non-streaming `Choice.message` as
    // `open_ai::RequestMessage`; its `Assistant` variant defines only
    // `reasoning_content`. The `reasoning` alias belongs exclusively to
    // `ResponseMessageDelta` and must not be inferred for full messages.
    // Source: contrib/zed/crates/open_ai/src/open_ai.rs.
    optional_string_extension(message, REASONING_CONTENT, "Chat response message")
        .map(|reasoning| reasoning.map(str::to_string))
}

pub(crate) fn stream_reasoning(payload: &Value) -> Result<Option<String>, String> {
    let Some(delta) = payload
        .pointer("/choices/0/delta")
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };

    // Zed's `open_ai::ResponseMessageDelta` defines both fields, and
    // `OpenAiEventMapper::map_event` emits thinking events from them in this
    // order. Commit 7187d65774 added `reasoning` specifically for common
    // OpenAI-compatible streamed thinking fields.
    // Sources: contrib/zed/crates/open_ai/src/{open_ai.rs,completion.rs}.
    let mut combined = String::new();
    for field in [REASONING, REASONING_CONTENT] {
        if let Some(reasoning) = optional_string_extension(delta, field, "Chat stream delta")? {
            combined.push_str(reasoning);
        }
    }
    Ok((!combined.is_empty()).then_some(combined))
}

fn optional_string_extension<'a>(
    object: &'a serde_json::Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<Option<&'a str>, String> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok((!value.is_empty()).then_some(value.as_str())),
        Some(_) => Err(format!("{context} `{field}` must be a string or null")),
    }
}

#[cfg(test)]
#[path = "compatibility_tests.rs"]
mod tests;
