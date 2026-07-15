//! Response-side Anthropic Messages constructors.
//!
//! Used by `* -> anthropic_messages` response and streaming translators to
//! build protocol-native response types (`ContentBlock`, `TextBlock`,
//! `ToolUseBlock`, ...) without repeating `None` / `DirectCaller` noise.

use serde_json::Value;

use crate::protocol::anthropic::messages as anthropic;

pub(crate) fn text_block(text: impl Into<String>) -> anthropic::TextBlock {
    anthropic::TextBlock {
        text: text.into(),
        citations: None.into(),
    }
}

pub(crate) fn tool_use_block(
    id: impl Into<String>,
    name: impl Into<String>,
    input: Value,
) -> anthropic::ToolUseBlock {
    anthropic::ToolUseBlock {
        id: id.into(),
        caller: anthropic::ToolCaller::Direct(anthropic::DirectCaller),
        input,
        name: name.into(),
    }
}

pub(crate) fn thinking_block(text: impl Into<String>) -> anthropic::ThinkingBlock {
    anthropic::ThinkingBlock {
        thinking: text.into(),
        signature: String::new(),
    }
}

pub(crate) fn redacted_thinking_block(data: impl Into<String>) -> anthropic::RedactedThinkingBlock {
    anthropic::RedactedThinkingBlock { data: data.into() }
}
