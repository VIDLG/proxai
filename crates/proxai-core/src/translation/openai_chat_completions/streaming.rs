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

use crate::json::deserialize_value;
use crate::protocol::openai::chat_completions::{
    ChatCompletionStreamRole, CreateChatCompletionStreamResponse,
};
use crate::translation::stream::{
    InboundStreamLifecycle, InboundStreamLifecyclePhase, StreamEnd, StreamIdentity,
    StreamTranslationError, StreamTranslationResult, StreamingPhase,
};

#[derive(Debug)]
pub(super) struct ChatInboundLifecycle<S, T> {
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
            pub(super) fn receive_terminal_finish(&mut self, terminal: T);
            pub(super) fn stop(&mut self);
            #[call(is_waiting)]
            pub(super) fn is_waiting_for_first_chunk(&self) -> bool;
            pub(super) fn is_stopped(&self) -> bool;
            pub(super) fn terminal(&self) -> Option<&T>;
            pub(super) fn terminal_mut(&mut self) -> Option<&mut T>;
            #[call(take_terminal)]
            pub(super) fn take_terminal_finish(
                &mut self,
                error: impl FnOnce() -> StreamTranslationError,
            ) -> StreamTranslationResult<T>;
        }
    }

    pub(super) fn parse_stream_event(
        &self,
        payload: Value,
    ) -> StreamTranslationResult<CreateChatCompletionStreamResponse> {
        let chunk = deserialize_value::<CreateChatCompletionStreamResponse>(
            &payload,
            "OpenAI Chat Completions stream event",
        )?;
        if let Some(role) = chunk
            .choices
            .iter()
            .find_map(|choice| choice.delta.role)
            .filter(|role| *role != ChatCompletionStreamRole::Assistant)
        {
            return Err(StreamTranslationError::Semantic(format!(
                "Chat stream emitted {role} role; cross-protocol translation requires an assistant choice"
            )));
        }
        Ok(chunk)
    }

    pub(super) fn register_chunk_stream(
        &mut self,
        identity: StreamIdentity,
        state: S,
    ) -> StreamTranslationResult<Option<StreamIdentity>> {
        if self.is_waiting_for_first_chunk() {
            self.inner.begin_streaming(identity.clone(), state);
            Ok(Some(identity))
        } else {
            self.ensure_same_stream_identity(&identity)?;
            if !matches!(
                self.inner.phase_kind(),
                InboundStreamLifecyclePhase::Streaming
            ) {
                return Err(StreamTranslationError::Semantic(format!(
                    "Chat stream emitted choice deltas while lifecycle was {}; expected streaming",
                    self.inner.phase_kind()
                )));
            }
            Ok(None)
        }
    }

    pub(super) fn streaming_phase_mut(
        &mut self,
    ) -> StreamTranslationResult<&mut StreamingPhase<S>> {
        self.inner
            .require_streaming_phase_mut("Chat", "choice deltas")
    }

    pub(super) fn take_streaming_phase(
        &mut self,
        error: impl FnOnce() -> StreamTranslationError,
    ) -> StreamTranslationResult<StreamingPhase<S>> {
        self.inner.take_streaming_phase(error)
    }

    pub(super) fn unexpected_stream_end_error(&self, end: StreamEnd) -> StreamTranslationError {
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
                        StreamEnd::Done => {
                            "Chat stream emitted [DONE] before a terminal finish_reason".to_string()
                        }
                        StreamEnd::Eof => {
                            "Chat stream reached EOF before a terminal finish_reason".to_string()
                        }
                    }
                } else {
                    "Chat stream completed without target-representable content, refusal, or function tool calls"
                        .to_string()
                }
            }
            InboundStreamLifecyclePhase::Terminal | InboundStreamLifecyclePhase::Stopped => {
                String::new()
            }
        };
        StreamTranslationError::Semantic(message)
    }

    pub(super) fn ensure_same_stream_identity(
        &self,
        chunk_identity: &StreamIdentity,
    ) -> StreamTranslationResult<()> {
        let identity = self.inner.require_identity(|| {
            StreamTranslationError::Semantic(
                "Chat stream identity is not initialized before assistant message chunk"
                    .to_string(),
            )
        })?;
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

pub(super) fn stream_identity(
    chunk: &CreateChatCompletionStreamResponse,
    id_prefix: &str,
) -> StreamIdentity {
    StreamIdentity::new(format!("{id_prefix}{}", chunk.id), chunk.model.clone())
}
