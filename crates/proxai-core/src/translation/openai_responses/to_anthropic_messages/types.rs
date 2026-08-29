//! Pair-local protocol-to-protocol conversions shared across the
//! `openai_responses -> anthropic_messages` pair's response and streaming
//! children.
//!
//! Request-only conversions stay in the `request` module; child-private
//! helpers stay in the consuming child module.

use crate::protocol::anthropic::messages::StopReason;
use crate::translation::openai_responses::stop::ResponsesStopKind;

impl From<ResponsesStopKind> for StopReason {
    fn from(value: ResponsesStopKind) -> Self {
        match value {
            ResponsesStopKind::EndTurn => Self::EndTurn,
            ResponsesStopKind::MaxTokens => Self::MaxTokens,
            ResponsesStopKind::ToolUse => Self::ToolUse,
            ResponsesStopKind::Refusal => Self::Refusal,
        }
    }
}
