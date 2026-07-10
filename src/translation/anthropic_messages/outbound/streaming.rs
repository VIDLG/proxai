//! Streaming Anthropic Messages constructors.
//!
//! Used by `* -> anthropic_messages` streaming translators to build
//! protocol-native `MessageStreamEvent` values. Carrier-level encoding stays in
//! `translation::streaming::typed_stream_events`.

use serde_json::{Map, Value};

use crate::protocol::anthropic::messages as anthropic;

use super::{text_block, thinking_block, tool_use_block};

pub(crate) fn message_start(
    id: String,
    model: String,
    usage: anthropic::Usage,
) -> anthropic::MessageStreamEvent {
    let mut event = anthropic::MessageStartEvent::new_empty_message(id, model);
    event.message.usage = usage;
    anthropic::MessageStreamEvent::MessageStart(event)
}

pub(crate) fn message_delta(
    delta: anthropic::MessageDelta,
    usage: anthropic::MessageDeltaUsage,
) -> anthropic::MessageStreamEvent {
    anthropic::MessageStreamEvent::MessageDelta(anthropic::MessageDeltaEvent { delta, usage })
}

pub(crate) fn message_stop() -> anthropic::MessageStreamEvent {
    anthropic::MessageStreamEvent::MessageStop(anthropic::MessageStopEvent)
}

pub(crate) fn content_block_start(
    index: u32,
    content_block: anthropic::ContentBlock,
) -> anthropic::MessageStreamEvent {
    anthropic::MessageStreamEvent::ContentBlockStart(anthropic::ContentBlockStartEvent {
        content_block,
        index,
    })
}

pub(crate) fn text_block_start(
    index: u32,
    text: impl Into<String>,
) -> anthropic::MessageStreamEvent {
    content_block_start(index, anthropic::ContentBlock::Text(text_block(text)))
}

pub(crate) fn thinking_block_start(index: u32) -> anthropic::MessageStreamEvent {
    content_block_start(
        index,
        anthropic::ContentBlock::Thinking(thinking_block(String::new())),
    )
}

pub(crate) fn tool_use_block_start(
    index: u32,
    id: String,
    name: String,
) -> anthropic::MessageStreamEvent {
    content_block_start(
        index,
        anthropic::ContentBlock::ToolUse(tool_use_block(id, name, Value::Object(Map::new()))),
    )
}

pub(crate) fn content_block_delta(
    index: u32,
    delta: anthropic::ContentBlockDelta,
) -> anthropic::MessageStreamEvent {
    anthropic::MessageStreamEvent::ContentBlockDelta(anthropic::ContentBlockDeltaEvent {
        delta,
        index,
    })
}

pub(crate) fn content_block_stop(index: u32) -> anthropic::MessageStreamEvent {
    anthropic::MessageStreamEvent::ContentBlockStop(anthropic::ContentBlockStopEvent { index })
}

pub(crate) fn text_delta(index: u32, text: String) -> anthropic::MessageStreamEvent {
    content_block_delta(
        index,
        anthropic::ContentBlockDelta::TextDelta(anthropic::TextDelta { text }),
    )
}

pub(crate) fn thinking_delta(index: u32, thinking: String) -> anthropic::MessageStreamEvent {
    content_block_delta(
        index,
        anthropic::ContentBlockDelta::ThinkingDelta(anthropic::ThinkingDelta { thinking }),
    )
}

pub(crate) fn input_json_delta(index: u32, partial_json: String) -> anthropic::MessageStreamEvent {
    content_block_delta(
        index,
        anthropic::ContentBlockDelta::InputJsonDelta(anthropic::InputJsonDelta { partial_json }),
    )
}
