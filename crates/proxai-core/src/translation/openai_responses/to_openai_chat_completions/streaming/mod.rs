//! `openai_responses -> openai_chat_completions` streaming translator.
//!
//! Converts Responses SSE events into Chat Completions stream chunks. Each
//! inbound `ResponseStreamEvent` is mapped to one or more
//! `CreateChatCompletionStreamResponse` chunks via target Chat outbound helpers,
//! then converted to carrier-level `StreamEvent`s at the boundary.
//!
//! Source-side lifecycle (identity, terminal detection, unexpected-end
//! reporting) is shared across all `responses -> *` translators through
//! `crate::translation::openai_responses::streaming::ResponsesInboundLifecycle`.

mod state;

use crate::protocol::openai::chat_completions::{CompletionUsage, FinishReason};
use crate::protocol::openai::responses::{
    OutputContent, OutputItem, Response, ResponseStreamEvent, ResponseUsage, SummaryPart,
};
use crate::translation::TranslationScope;
use crate::translation::openai_chat_completions::compatibility::inject_stream_reasoning;
use crate::translation::openai_chat_completions::outbound::{
    assistant_role_delta as message_start_delta, chat_choice_chunk, chat_usage_chunk,
    refusal_delta, text_delta, tool_arguments_delta, tool_call_start_delta,
};
use crate::translation::openai_responses::streaming::{
    ResponsesInboundLifecycle, ResponsesOutputSegmentKey, response_failure_error,
};
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamIdentity, StreamTranslationError, StreamTranslationResult,
};

use state::StreamingState;

fn reasoning_stream_event(
    identity: &StreamIdentity,
    reasoning: String,
) -> StreamTranslationResult<StreamEvent> {
    let mut payload = serde_json::to_value(chat_choice_chunk(identity, Default::default(), None))?;
    inject_stream_reasoning(&mut payload, reasoning)?;
    StreamEvent::message(payload)
}

#[derive(Debug, Default)]
pub(crate) struct ResponsesToChatStreaming {
    lifecycle: ResponsesInboundLifecycle<StreamingState>,
}

impl ResponsesToChatStreaming {
    pub(crate) fn translate_event(
        &mut self,
        event: StreamEvent,
        scope: &TranslationScope,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let parsed = self.lifecycle.parse_stream_event(event.data)?;
        let event_type = parsed.as_ref().to_string();
        let mut chunks = Vec::new();

        match parsed {
            ResponseStreamEvent::ResponseCreated(event) => {
                chunks.extend(self.project_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseInProgress(event) => {
                chunks.extend(self.project_response_snapshot(&event.response)?);
            }
            ResponseStreamEvent::ResponseOutputItemAdded(event) => match event.item {
                OutputItem::FunctionCall(call) => {
                    let key = ResponsesOutputSegmentKey::FunctionArguments {
                        output_index: event.output_index,
                    };
                    let state = self.lifecycle.streaming_state_mut()?;
                    let tool_call_index = state.register_tool_call(event.output_index);
                    state.append_content(key, &call.arguments);

                    let identity = self.lifecycle.stream_identity()?.clone();
                    chunks.push(StreamEvent::message(chat_choice_chunk(
                        &identity,
                        tool_call_start_delta(
                            tool_call_index,
                            call.call_id,
                            call.name,
                            None,
                            call.arguments,
                        ),
                        None,
                    ))?);
                }
                OutputItem::Message(_) | OutputItem::Reasoning(_) => {}
                other => scope.dropped(
                    format!(
                        "Responses output item `{}` at index {}",
                        other.as_ref(),
                        event.output_index
                    ),
                    "Responses output item has no Chat Completions streaming representation",
                ),
            },
            ResponseStreamEvent::ResponseContentPartAdded(event) => match event.part {
                OutputContent::OutputText(part) => {
                    self.lifecycle.streaming_state_mut()?.append_content(
                        ResponsesOutputSegmentKey::Text {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &part.text,
                    );
                    if let Some(chunk) = self.emit_text_chunk(part.text)? {
                        chunks.push(chunk);
                    }
                }
                OutputContent::Refusal(part) => {
                    self.lifecycle.streaming_state_mut()?.append_content(
                        ResponsesOutputSegmentKey::Refusal {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &part.refusal,
                    );
                    if let Some(chunk) = self.emit_refusal_chunk(part.refusal)? {
                        chunks.push(chunk);
                    }
                }
                OutputContent::ReasoningText(part) => {
                    self.lifecycle.streaming_state_mut()?.append_content(
                        ResponsesOutputSegmentKey::ReasoningText {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &part.text,
                    );
                    if let Some(chunk) = self.emit_reasoning_chunk(part.text)? {
                        chunks.push(chunk);
                    }
                }
            },
            ResponseStreamEvent::ResponseOutputTextDelta(event) => {
                self.lifecycle.streaming_state_mut()?.append_content(
                    ResponsesOutputSegmentKey::Text {
                        output_index: event.output_index,
                        content_index: event.content_index,
                    },
                    &event.delta,
                );
                if let Some(chunk) = self.emit_text_chunk(event.delta)? {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseOutputTextDone(event) => {
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(
                        ResponsesOutputSegmentKey::Text {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &event.text,
                        &event_type,
                    )?;
                if let Some(suffix) = suffix
                    && let Some(chunk) = self.emit_text_chunk(suffix)?
                {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseRefusalDelta(event) => {
                self.lifecycle.streaming_state_mut()?.append_content(
                    ResponsesOutputSegmentKey::Refusal {
                        output_index: event.output_index,
                        content_index: event.content_index,
                    },
                    &event.delta,
                );
                if let Some(chunk) = self.emit_refusal_chunk(event.delta)? {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseRefusalDone(event) => {
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(
                        ResponsesOutputSegmentKey::Refusal {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &event.refusal,
                        &event_type,
                    )?;
                if let Some(suffix) = suffix
                    && let Some(chunk) = self.emit_refusal_chunk(suffix)?
                {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseReasoningTextDelta(event) => {
                self.lifecycle.streaming_state_mut()?.append_content(
                    ResponsesOutputSegmentKey::ReasoningText {
                        output_index: event.output_index,
                        content_index: event.content_index,
                    },
                    &event.delta,
                );
                if let Some(chunk) = self.emit_reasoning_chunk(event.delta)? {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseReasoningTextDone(event) => {
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(
                        ResponsesOutputSegmentKey::ReasoningText {
                            output_index: event.output_index,
                            content_index: event.content_index,
                        },
                        &event.text,
                        &event_type,
                    )?;
                if let Some(suffix) = suffix
                    && let Some(chunk) = self.emit_reasoning_chunk(suffix)?
                {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseContentPartDone(event) => match event.part {
                OutputContent::OutputText(part) => {
                    let suffix = self
                        .lifecycle
                        .streaming_state_mut()?
                        .reconcile_content_snapshot(
                            ResponsesOutputSegmentKey::Text {
                                output_index: event.output_index,
                                content_index: event.content_index,
                            },
                            &part.text,
                            &event_type,
                        )?;
                    if let Some(suffix) = suffix
                        && let Some(chunk) = self.emit_text_chunk(suffix)?
                    {
                        chunks.push(chunk);
                    }
                }
                OutputContent::Refusal(part) => {
                    let suffix = self
                        .lifecycle
                        .streaming_state_mut()?
                        .reconcile_content_snapshot(
                            ResponsesOutputSegmentKey::Refusal {
                                output_index: event.output_index,
                                content_index: event.content_index,
                            },
                            &part.refusal,
                            &event_type,
                        )?;
                    if let Some(suffix) = suffix
                        && let Some(chunk) = self.emit_refusal_chunk(suffix)?
                    {
                        chunks.push(chunk);
                    }
                }
                OutputContent::ReasoningText(part) => {
                    let suffix = self
                        .lifecycle
                        .streaming_state_mut()?
                        .reconcile_content_snapshot(
                            ResponsesOutputSegmentKey::ReasoningText {
                                output_index: event.output_index,
                                content_index: event.content_index,
                            },
                            &part.text,
                            &event_type,
                        )?;
                    if let Some(suffix) = suffix
                        && let Some(chunk) = self.emit_reasoning_chunk(suffix)?
                    {
                        chunks.push(chunk);
                    }
                }
            },
            ResponseStreamEvent::ResponseReasoningSummaryPartAdded(event) => {
                let SummaryPart::SummaryText(part) = event.part;
                self.lifecycle.streaming_state_mut()?.append_content(
                    ResponsesOutputSegmentKey::ReasoningSummary {
                        output_index: event.output_index,
                        summary_index: event.summary_index,
                    },
                    &part.text,
                );
                if let Some(chunk) = self.emit_reasoning_chunk(part.text)? {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDelta(event) => {
                self.lifecycle.streaming_state_mut()?.append_content(
                    ResponsesOutputSegmentKey::ReasoningSummary {
                        output_index: event.output_index,
                        summary_index: event.summary_index,
                    },
                    &event.delta,
                );
                if let Some(chunk) = self.emit_reasoning_chunk(event.delta)? {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryTextDone(event) => {
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(
                        ResponsesOutputSegmentKey::ReasoningSummary {
                            output_index: event.output_index,
                            summary_index: event.summary_index,
                        },
                        &event.text,
                        &event_type,
                    )?;
                if let Some(suffix) = suffix
                    && let Some(chunk) = self.emit_reasoning_chunk(suffix)?
                {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseReasoningSummaryPartDone(event) => {
                let SummaryPart::SummaryText(part) = event.part;
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(
                        ResponsesOutputSegmentKey::ReasoningSummary {
                            output_index: event.output_index,
                            summary_index: event.summary_index,
                        },
                        &part.text,
                        &event_type,
                    )?;
                if let Some(suffix) = suffix
                    && let Some(chunk) = self.emit_reasoning_chunk(suffix)?
                {
                    chunks.push(chunk);
                }
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(event) => {
                let key = ResponsesOutputSegmentKey::FunctionArguments {
                    output_index: event.output_index,
                };
                let tool_call_index = {
                    let state = self.lifecycle.streaming_state_mut()?;
                    state.append_content(key, &event.delta);
                    state.require_tool_call_index(event.output_index)?
                };
                if !event.delta.is_empty() {
                    let identity = self.lifecycle.stream_identity()?.clone();
                    chunks.push(StreamEvent::message(chat_choice_chunk(
                        &identity,
                        tool_arguments_delta(tool_call_index, event.delta),
                        None,
                    ))?);
                }
            }
            ResponseStreamEvent::ResponseFunctionCallArgumentsDone(event) => {
                let key = ResponsesOutputSegmentKey::FunctionArguments {
                    output_index: event.output_index,
                };
                let suffix = self
                    .lifecycle
                    .streaming_state_mut()?
                    .reconcile_content_snapshot(key, &event.arguments, &event_type)?;
                if let Some(arguments) = suffix {
                    let tool_call_index = self
                        .lifecycle
                        .streaming_state_mut()?
                        .require_tool_call_index(event.output_index)?;
                    let identity = self.lifecycle.stream_identity()?.clone();
                    chunks.push(StreamEvent::message(chat_choice_chunk(
                        &identity,
                        tool_arguments_delta(tool_call_index, arguments),
                        None,
                    ))?);
                }
            }
            ResponseStreamEvent::ResponseOutputItemDone(_) => {}
            ResponseStreamEvent::ResponseCompleted(event) => {
                self.require_representable_content()?;
                let usage = event.response.usage;
                let finish_reason = self.completion_finish_reason()?;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(finish_reason, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseIncomplete(event) => {
                self.require_representable_content()?;
                let usage = event.response.usage;
                self.lifecycle.receive_terminal_event()?;
                chunks.extend(self.terminal_chunks(FinishReason::Length, usage)?);
                self.lifecycle.stop();
            }
            ResponseStreamEvent::ResponseFailed(event) => {
                return Err(response_failure_error(&event.response));
            }
            ResponseStreamEvent::ResponseError(event) => {
                return Err(StreamTranslationError::Semantic(format!(
                    "Responses stream error{}: {}",
                    event
                        .code
                        .as_non_null()
                        .map(String::as_str)
                        .map(|code| format!(" ({code})"))
                        .unwrap_or_default(),
                    event.message
                )));
            }
            other => scope.dropped(
                format!("Responses stream event `{}`", other.as_ref()),
                "Responses stream event has no Chat Completions representation",
            ),
        }

        Ok(chunks)
    }

    pub(crate) fn finish_stream(
        &mut self,
        end: StreamEnd,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        self.lifecycle.finish_stream(end)?;
        Ok(Vec::new())
    }
}

impl ResponsesToChatStreaming {
    fn project_response_snapshot(
        &mut self,
        response: &Response,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let identity = StreamIdentity::new(response.id.clone(), response.model.clone());
        self.lifecycle
            .ensure_response_stream(identity.clone(), StreamingState::default())?;
        if self.lifecycle.streaming_state_mut()?.mark_message_started() {
            Ok(vec![StreamEvent::message(chat_choice_chunk(
                &identity,
                message_start_delta(),
                None,
            ))?])
        } else {
            Ok(Vec::new())
        }
    }

    fn emit_text_chunk(&mut self, text: String) -> StreamTranslationResult<Option<StreamEvent>> {
        if text.is_empty() {
            return Ok(None);
        }
        self.lifecycle.streaming_state_mut()?.mark_content();
        let identity = self.lifecycle.stream_identity()?.clone();
        StreamEvent::message(chat_choice_chunk(&identity, text_delta(text), None)).map(Some)
    }

    fn emit_refusal_chunk(
        &mut self,
        refusal: String,
    ) -> StreamTranslationResult<Option<StreamEvent>> {
        if refusal.is_empty() {
            return Ok(None);
        }
        self.lifecycle.streaming_state_mut()?.mark_content();
        let identity = self.lifecycle.stream_identity()?.clone();
        StreamEvent::message(chat_choice_chunk(&identity, refusal_delta(refusal), None)).map(Some)
    }

    fn emit_reasoning_chunk(
        &mut self,
        reasoning: String,
    ) -> StreamTranslationResult<Option<StreamEvent>> {
        if reasoning.is_empty() {
            return Ok(None);
        }
        self.lifecycle.streaming_state_mut()?.mark_content();
        let identity = self.lifecycle.stream_identity()?.clone();
        reasoning_stream_event(&identity, reasoning).map(Some)
    }

    fn completion_finish_reason(&mut self) -> StreamTranslationResult<FinishReason> {
        Ok(if self.lifecycle.streaming_state_mut()?.has_tool_calls() {
            FinishReason::ToolCalls
        } else {
            FinishReason::Stop
        })
    }

    fn require_representable_content(&mut self) -> StreamTranslationResult<()> {
        if self
            .lifecycle
            .streaming_state_mut()?
            .has_representable_output()
        {
            Ok(())
        } else {
            Err(StreamTranslationError::Semantic(
                "Responses stream completed without Chat-representable text, refusal, reasoning, or function tool calls"
                    .to_string(),
            ))
        }
    }

    /// Build the terminal chunk sequence for a finished Responses turn:
    /// an empty delta carrying the finish reason, an optional usage chunk, and
    /// the carrier-level `[DONE]`.
    fn terminal_chunks(
        &self,
        finish_reason: FinishReason,
        usage: Option<ResponseUsage>,
    ) -> StreamTranslationResult<Vec<StreamEvent>> {
        let identity = self.lifecycle.stream_identity()?.clone();
        let mut chunks = vec![StreamEvent::message(chat_choice_chunk(
            &identity,
            Default::default(),
            Some(finish_reason),
        ))?];

        if let Some(usage) = usage {
            chunks.push(StreamEvent::message(chat_usage_chunk(
                &identity,
                CompletionUsage::from(&usage),
            ))?);
        }

        chunks.push(StreamEvent::done());
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests;
