//! Pure output builders for
//! `openai_chat_completions -> anthropic_messages` streaming translation.
//!
//! `message_delta` builds the terminal Anthropic message-delta event from Chat
//! terminal state.

use crate::protocol::anthropic::messages::{MessageDelta, MessageStreamEvent};
use crate::translation::anthropic_messages::outbound::message_delta as message_delta_event;

use super::super::response::chat_stop_state;
use super::state::PendingAnthropicTerminal;

pub(super) fn message_delta(terminal: &PendingAnthropicTerminal) -> MessageStreamEvent {
    let usage = terminal.usage.as_ref().map(Into::into).unwrap_or_default();
    let stop = chat_stop_state(
        (!terminal.refusal.is_empty()).then_some(terminal.refusal.as_str()),
        Some(terminal.finish_reason),
    );
    message_delta_event(
        MessageDelta {
            container: None,
            stop_details: stop.details,
            stop_reason: stop.reason,
            stop_sequence: stop.sequence,
        },
        usage,
    )
}
