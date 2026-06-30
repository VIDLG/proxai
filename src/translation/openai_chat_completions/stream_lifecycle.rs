//! Inbound-side streaming lifecycle shared by translators rooted at
//! `openai_chat_completions`.
//!
//! Chat Completions streaming is less formally framed than Anthropic Messages,
//! but every target still needs the same source-protocol checks:
//!
//! - the stream must start with a semantic assistant chunk before it can finish,
//! - all chunks must keep the same source id/model,
//! - a terminal `finish_reason` closes semantic content before `[DONE]`/EOF,
//! - usage-only chunks are only valid after terminal content for targets that
//!   consume them.
//!
//! Target-specific private state stays in the pair translator; while streaming,
//! the lifecycle wraps it in `StreamingPhase` so output-progress tracking is
//! shared across target protocols.

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;
use crate::translation::streaming::{
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

#[derive(Debug, Default)]
pub(crate) enum ChatInboundLifecycle<S, T> {
    #[default]
    WaitingForFirstChunk,
    Streaming(StreamingPhase<S>),
    ReceivedTerminalFinish(T),
    Stopped,
}

impl<S, T> ChatInboundLifecycle<S, T> {
    pub(crate) fn begin_streaming(&mut self, state: S) {
        *self = Self::Streaming(StreamingPhase::new(state));
    }

    pub(crate) fn streaming_phase_mut(
        &mut self,
        target_protocol_label: &'static str,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        match self {
            Self::Streaming(phase) => Ok(phase),
            Self::WaitingForFirstChunk => Err(StreamTranslationError::Semantic(format!(
                "Chat stream emitted choice deltas before the {target_protocol_label} message was initialized"
            ))),
            Self::ReceivedTerminalFinish(_) => Err(StreamTranslationError::Semantic(
                "Chat stream emitted choice deltas after a terminal finish_reason".to_string(),
            )),
            Self::Stopped => Err(StreamTranslationError::Semantic(format!(
                "Chat stream emitted choice deltas after the {target_protocol_label} message was stopped"
            ))),
        }
    }

    pub(crate) fn unexpected_stream_end_error(
        &self,
        end: SseStreamEnd,
        target_protocol_label: &'static str,
    ) -> StreamTranslationError {
        let end_label = match end {
            SseStreamEnd::DoneSentinel => "[DONE]",
            SseStreamEnd::Eof => "EOF",
        };

        let message = match self {
            Self::WaitingForFirstChunk => {
                format!("Chat stream reached {end_label} before any assistant message chunk")
            }
            Self::Streaming(phase) if phase.emitted_any() => match end {
                SseStreamEnd::DoneSentinel => {
                    "Chat stream emitted [DONE] before a terminal finish_reason".to_string()
                }
                SseStreamEnd::Eof => {
                    "Chat stream reached EOF before a terminal finish_reason".to_string()
                }
            },
            Self::Streaming(_) => format!(
                "Chat stream completed without {}-representable content, refusal, or function tool calls",
                target_protocol_label
            ),
            Self::ReceivedTerminalFinish(_) => String::new(),
            Self::Stopped => String::new(),
        };
        StreamTranslationError::Semantic(message)
    }
}

pub(crate) fn stream_identity(
    chunk: &CreateChatCompletionStreamResponse,
    id_prefix: &str,
) -> StreamIdentity {
    StreamIdentity::new(format!("{id_prefix}{}", chunk.id), chunk.model.clone())
}

pub(crate) fn ensure_same_stream_identity(
    identity: &StreamIdentity,
    chunk: &CreateChatCompletionStreamResponse,
    id_prefix: &str,
) -> StreamTranslationResult<()> {
    let chunk_identity = stream_identity(chunk, id_prefix);
    if identity.id() != chunk_identity.id() {
        return Err(StreamTranslationError::Semantic(format!(
            "Chat stream changed id from {} to {}",
            identity.id(),
            chunk_identity.id()
        )));
    }
    if identity.model() != chunk_identity.model() {
        return Err(StreamTranslationError::Semantic(format!(
            "Chat stream changed model from {} to {}",
            identity.model(),
            chunk_identity.model()
        )));
    }
    Ok(())
}
