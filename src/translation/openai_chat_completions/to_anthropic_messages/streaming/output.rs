//! Pure output builders for
//! `openai_chat_completions -> anthropic_messages` streaming translation.
//!
//! `encode_outputs` maps finalized Anthropic `MessageStreamEvent`s to
//! carrier-level `StreamEvent`s. `message_delta` builds the terminal
//! Anthropic message-delta event from Chat terminal state.

use crate::protocol::anthropic::messages::{
    MessageDelta, MessageDeltaEvent, MessageDeltaUsage, MessageStopEvent, MessageStreamEvent,
};
use crate::protocol::openai::chat_completions::CompletionUsage;

use crate::translation::streaming::{StreamEvent, StreamTranslationResult};

use super::super::response::chat_stop_state;
use super::state::ChatTerminalState;

/// Map finalized Anthropic events to carrier-level `StreamEvent`s.
///
/// Each `MessageStreamEvent` carries its SSE event type via `strum::AsRefStr`
/// (`as_ref()` returns the event-type string, e.g. `"message_start"`); the
/// event payload is serialized to JSON by the carrier.
pub(super) fn encode_outputs(
    outputs: Vec<MessageStreamEvent>,
) -> StreamTranslationResult<Vec<StreamEvent>> {
    outputs
        .into_iter()
        .map(|event| {
            let event_type = event.as_ref().to_string();
            StreamEvent::json(event_type, event)
        })
        .collect()
}

pub(super) fn message_delta(
    terminal: &ChatTerminalState,
    usage: Option<&CompletionUsage>,
) -> MessageStreamEvent {
    let usage: crate::protocol::anthropic::messages::Usage =
        usage.map(Into::into).unwrap_or_default();
    let stop = chat_stop_state(
        (!terminal.refusal.is_empty()).then_some(terminal.refusal.as_str()),
        Some(terminal.finish_reason),
    );
    MessageStreamEvent::MessageDelta(MessageDeltaEvent {
        delta: MessageDelta {
            container: None,
            stop_details: stop.details,
            stop_reason: stop.reason,
            stop_sequence: stop.sequence,
        },
        usage: MessageDeltaUsage {
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            cache_read_input_tokens: usage.cache_read_input_tokens,
            input_tokens: Some(usage.input_tokens),
            output_tokens: usage.output_tokens,
            output_tokens_details: usage.output_tokens_details,
            server_tool_use: usage.server_tool_use,
        },
    })
}

/// Terminal Anthropic events emitted to close a message stream:
/// `message_delta` (carrying stop reason + usage) followed by `message_stop`.
///
/// `usage` is `Some` only when the caller has a final usage snapshot
/// (typically from a trailing usage-only Chat chunk). When `None`, the
/// terminal usage fields are populated from the terminal state's accumulated
/// values (which may be zero if usage never arrived).
pub(super) fn terminal_events(
    terminal: &ChatTerminalState,
    usage: Option<&CompletionUsage>,
) -> Vec<MessageStreamEvent> {
    vec![
        message_delta(terminal, usage),
        MessageStreamEvent::MessageStop(MessageStopEvent),
    ]
}
