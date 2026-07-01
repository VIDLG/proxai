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

use delegate::delegate;
use serde_json::Value;

use crate::protocol::openai::chat_completions::CreateChatCompletionStreamResponse;
use crate::sse::SseEvent;
use crate::translation::streaming::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, RequireStreamingPhaseContext,
    SseStreamEnd, StreamIdentity, StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

#[derive(Debug)]
pub(crate) struct ChatInboundLifecycle<S, T> {
    inner: InboundStreamLifecycle<S, T>,
}

impl<S, T> Default for ChatInboundLifecycle<S, T> {
    fn default() -> Self {
        Self {
            inner: InboundStreamLifecycle::default(),
        }
    }
}

impl<S, T> ChatInboundLifecycle<S, T> {
    delegate! {
        to self.inner {
            #[call(receive_terminal)]
            pub(crate) fn receive_terminal_finish(&mut self, terminal: T);
            pub(crate) fn stop(&mut self);
            #[call(is_waiting)]
            pub(crate) fn is_waiting_for_first_chunk(&self) -> bool;
            pub(crate) fn is_stopped(&self) -> bool;
            pub(crate) fn is_terminal(&self) -> bool;
            pub(crate) fn terminal(&self) -> Option<&T>;
            pub(crate) fn terminal_mut(&mut self) -> Option<&mut T>;
            pub(crate) fn streaming_phase(&self) -> Option<&StreamingPhase<S>>;
            #[call(take_terminal)]
            pub(crate) fn take_terminal_finish(
                &mut self,
                error: impl FnOnce() -> StreamTranslationError,
            ) -> StreamTranslationResult<T>;
        }
    }

    pub(crate) fn parse_stream_event(
        &self,
        event: SseEvent,
    ) -> StreamTranslationResult<(Value, CreateChatCompletionStreamResponse)> {
        let payload = event.payload_with_type()?;
        let chunk = serde_json::from_value::<CreateChatCompletionStreamResponse>(payload.clone())?;
        Ok((payload, chunk))
    }

    pub(crate) fn begin_chunk_stream(
        &mut self,
        identity: StreamIdentity,
        state: S,
    ) -> StreamTranslationResult<()> {
        if !self.inner.is_waiting() {
            return Err(StreamTranslationError::Semantic(
                "Chat stream emitted duplicate assistant message chunk".to_string(),
            ));
        }
        self.inner.begin_streaming(identity, state);
        Ok(())
    }

    pub(crate) fn streaming_phase_mut(
        &mut self,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        self.inner
            .require_streaming_phase_mut(RequireStreamingPhaseContext {
                source: "Chat",
                event: "choice deltas",
            })
    }

    pub(crate) fn take_streaming_phase(
        &mut self,
        error: impl FnOnce() -> StreamTranslationError,
    ) -> StreamTranslationResult<StreamingPhase<S>> {
        self.inner.take_streaming_phase(error)
    }

    pub(crate) fn unexpected_stream_end_error(
        &self,
        end: SseStreamEnd,
        target_protocol_label: &'static str,
    ) -> StreamTranslationError {
        let message = match self.inner.phase_kind() {
            InboundStreamLifecyclePhase::Waiting => {
                format!("Chat stream reached {end} before any assistant message chunk")
            }
            InboundStreamLifecyclePhase::Streaming => {
                let phase = self
                    .inner
                    .streaming_phase()
                    .expect("streaming phase exists");
                if phase.emitted_any() {
                    match end {
                        SseStreamEnd::DoneSentinel => {
                            "Chat stream emitted [DONE] before a terminal finish_reason".to_string()
                        }
                        SseStreamEnd::Eof => {
                            "Chat stream reached EOF before a terminal finish_reason".to_string()
                        }
                    }
                } else {
                    format!(
                        "Chat stream completed without {target_protocol_label}-representable content, refusal, or function tool calls"
                    )
                }
            }
            InboundStreamLifecyclePhase::Terminal | InboundStreamLifecyclePhase::Stopped => {
                String::new()
            }
        };
        StreamTranslationError::Semantic(message)
    }

    pub(crate) fn ensure_same_stream_identity(
        &self,
        chunk: &CreateChatCompletionStreamResponse,
        id_prefix: &str,
        uninitialized_message: &'static str,
    ) -> StreamTranslationResult<()> {
        let identity = self.inner.require_identity(|| {
            StreamTranslationError::Semantic(uninitialized_message.to_string())
        })?;
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
}

pub(crate) fn stream_identity(
    chunk: &CreateChatCompletionStreamResponse,
    id_prefix: &str,
) -> StreamIdentity {
    StreamIdentity::new(format!("{id_prefix}{}", chunk.id), chunk.model.clone())
}
