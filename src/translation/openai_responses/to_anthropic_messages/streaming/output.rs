//! Output builders for
//! `openai_responses -> anthropic_messages` streaming translation.
//!
//! The translator emits Anthropic protocol-native `MessageStreamEvent` values
//! here, then maps them to carrier-level `StreamEvent` only at the boundary.

use serde_json::{Map, Value};

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent,
    ContentBlockStopEvent, InputJsonDelta, MessageDelta, MessageDeltaEvent, MessageDeltaUsage,
    MessageStartEvent, MessageStopEvent, MessageStreamEvent, StopReason, TextDelta, ThinkingDelta,
    Usage,
};
use crate::translation::anthropic_messages::outbound::{
    text_block, thinking_block, tool_use_block,
};
use crate::translation::streaming::{StreamEvent, StreamTranslationResult};

pub(super) fn stream_event(event: MessageStreamEvent) -> StreamTranslationResult<StreamEvent> {
    let event_type = event.as_ref().to_string();
    StreamEvent::json(event_type, event)
}

pub(super) fn message_start(id: String, model: String, input_tokens: u32) -> MessageStreamEvent {
    let mut event = MessageStartEvent::new_empty_message(id, model);
    event.message.usage = initial_usage(input_tokens);
    MessageStreamEvent::MessageStart(event)
}

pub(super) fn text_block_start(index: u32) -> MessageStreamEvent {
    content_block_start(index, ContentBlock::Text(text_block(String::new())))
}

pub(super) fn thinking_block_start(index: u32) -> MessageStreamEvent {
    content_block_start(index, ContentBlock::Thinking(thinking_block(String::new())))
}

pub(super) fn tool_use_block_start(index: u32, id: String, name: String) -> MessageStreamEvent {
    content_block_start(
        index,
        ContentBlock::ToolUse(tool_use_block(id, name, Value::Object(Map::new()))),
    )
}

pub(super) fn text_delta(index: u32, text: String) -> MessageStreamEvent {
    content_block_delta(index, ContentBlockDelta::TextDelta(TextDelta { text }))
}

pub(super) fn thinking_delta(index: u32, thinking: String) -> MessageStreamEvent {
    content_block_delta(
        index,
        ContentBlockDelta::ThinkingDelta(ThinkingDelta { thinking }),
    )
}

pub(super) fn input_json_delta(index: u32, partial_json: String) -> MessageStreamEvent {
    content_block_delta(
        index,
        ContentBlockDelta::InputJsonDelta(InputJsonDelta { partial_json }),
    )
}

pub(super) fn content_block_stop(index: u32) -> MessageStreamEvent {
    MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index })
}

pub(super) fn message_delta(
    stop_reason: StopReason,
    input_tokens: u32,
    output_tokens: u32,
) -> MessageStreamEvent {
    MessageStreamEvent::MessageDelta(MessageDeltaEvent {
        delta: MessageDelta {
            container: None,
            stop_details: None,
            stop_reason: Some(stop_reason),
            stop_sequence: None,
        },
        usage: MessageDeltaUsage {
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            input_tokens: Some(input_tokens),
            output_tokens,
            output_tokens_details: None,
            server_tool_use: None,
        },
    })
}

pub(super) fn message_stop() -> MessageStreamEvent {
    MessageStreamEvent::MessageStop(MessageStopEvent)
}

fn content_block_start(index: u32, content_block: ContentBlock) -> MessageStreamEvent {
    MessageStreamEvent::ContentBlockStart(ContentBlockStartEvent {
        content_block,
        index,
    })
}

fn content_block_delta(index: u32, delta: ContentBlockDelta) -> MessageStreamEvent {
    MessageStreamEvent::ContentBlockDelta(ContentBlockDeltaEvent { delta, index })
}

fn initial_usage(input_tokens: u32) -> Usage {
    Usage {
        input_tokens,
        output_tokens: 0,
        ..Usage::default()
    }
}

pub(super) fn encode_events(
    events: Vec<MessageStreamEvent>,
) -> StreamTranslationResult<Vec<StreamEvent>> {
    events.into_iter().map(stream_event).collect()
}
