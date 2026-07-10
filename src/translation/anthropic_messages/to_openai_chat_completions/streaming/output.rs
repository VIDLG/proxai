//! Pure output builders for
//! `anthropic_messages -> openai_chat_completions` streaming translation.
//!
//! These constructors take already-decided protocol values (identity,
//! deltas, finish reason) and assemble Chat Completions stream response
//! payloads. They own no streaming state.

use crate::protocol::anthropic::messages::{MessageDelta, StopReason};

pub(super) fn chat_terminal_delta(delta: MessageDelta, emitted_text: bool) -> Option<String> {
    // MessageDelta.stop_reason is converted by the caller into Chat's
    // choice-level `finish_reason`; Chat stream deltas have no field for
    // Anthropic `container` or `stop_sequence`.
    //
    // Non-streaming refusal conversion can move final text into
    // `message.refusal` and leave `message.content` empty. Streaming cannot
    // retract text deltas that were already sent without buffering the whole
    // response, so only emit `delta.refusal` when no text content has been
    // streamed yet.
    if emitted_text || !matches!(delta.stop_reason, Some(StopReason::Refusal)) {
        return None;
    }

    let stop_details = delta.stop_details?;
    stop_details.explanation
}
