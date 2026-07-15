use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_stream::stream;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use strum::AsRefStr;

use crate::http_support::{ByteStream, ByteStreamError};
use crate::sse::{SseError, SseEventScanner, done_sentinel_bytes, encode_sse_json};
use crate::translation::Translator;
use crate::translation::stream::{
    StreamEnd, StreamEvent, StreamTranslationError, StreamTranslationInput, StreamTranslationResult,
};

pub(crate) type SseTranslationStream =
    Pin<Box<dyn Stream<Item = Result<Bytes, SseTranslationStreamError>> + Send + 'static>>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SseTranslationStreamError {
    #[error("upstream SSE stream error: {0}")]
    Upstream(ByteStreamError),

    #[error(transparent)]
    Translation(#[from] StreamTranslationFailure),

    #[error("translated SSE event encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum StreamTranslatorErrorStage {
    #[default]
    Event,
    Finish,
}

impl StreamTranslatorErrorStage {
    fn error_prefix(self) -> &'static str {
        match self {
            Self::Event => "stream translation error",
            Self::Finish => "stream translation finish error",
        }
    }
}

/// The upstream SSE frame that directly triggered, or immediately preceded, a
/// stream translation failure. Its data is retained only for local diagnostics.
#[derive(Debug, Clone)]
pub(crate) struct UpstreamSseEvent {
    pub(crate) event_type: String,
    pub(crate) data: String,
}

/// Application-carrier context for a semantic stream translation failure.
#[derive(Debug, thiserror::Error)]
#[error("{}: {error}", stage.error_prefix())]
pub(crate) struct StreamTranslationFailure {
    pub(crate) stage: StreamTranslatorErrorStage,
    pub(crate) error: StreamTranslationError,
    pub(crate) upstream_event: Option<UpstreamSseEvent>,
    pub(crate) end: Option<StreamEnd>,
}

impl StreamTranslationFailure {
    fn new(
        stage: StreamTranslatorErrorStage,
        error: StreamTranslationError,
        upstream_event: Option<UpstreamSseEvent>,
        end: Option<StreamEnd>,
    ) -> Self {
        Self {
            stage,
            error,
            upstream_event,
            end,
        }
    }
}

#[derive(Default)]
struct SseInputContext {
    stage: StreamTranslatorErrorStage,
    upstream_event: Option<UpstreamSseEvent>,
    end: Option<StreamEnd>,
    pending_error: Option<SseTranslationStreamError>,
}

impl SseInputContext {
    fn observe_event(&mut self, event: UpstreamSseEvent) {
        self.stage = StreamTranslatorErrorStage::Event;
        self.upstream_event = Some(event);
        self.end = None;
    }

    fn observe_end(&mut self, end: StreamEnd) {
        self.stage = StreamTranslatorErrorStage::Finish;
        self.end = Some(end);
    }

    fn translation_failure(&self, error: StreamTranslationError) -> StreamTranslationFailure {
        StreamTranslationFailure::new(self.stage, error, self.upstream_event.clone(), self.end)
    }
}

/// Adapt raw upstream SSE bytes to the core structured-stream API.
///
/// Translation failures remain structured stream errors. The application layer
/// retains raw carrier context for diagnostics and decides how to represent
/// failures to the HTTP client.
pub(crate) fn translate_sse_stream(
    input: ByteStream,
    translator: Translator,
) -> SseTranslationStream {
    let context = Arc::new(Mutex::new(SseInputContext::default()));
    let translation_input = decode_sse_stream(input, context.clone());
    let translated = translator.translate_stream(translation_input);

    Box::pin(stream! {
        futures_util::pin_mut!(translated);

        while let Some(item) = translated.next().await {
            if let Some(error) = take_pending_error(&context) {
                yield Err(error);
                return;
            }

            match item {
                Ok(event) => match encode_stream_event(&event) {
                    Ok(chunk) => yield Ok(chunk),
                    Err(error) => {
                        yield Err(error.into());
                        return;
                    }
                },
                Err(error) => {
                    let failure = context
                        .lock()
                        .expect("SSE input context lock poisoned")
                        .translation_failure(error);
                    yield Err(failure.into());
                    return;
                }
            }
        }

        if let Some(error) = take_pending_error(&context) {
            yield Err(error);
        }
    })
}

fn decode_sse_stream(
    input: ByteStream,
    context: Arc<Mutex<SseInputContext>>,
) -> impl Stream<Item = StreamTranslationResult<StreamTranslationInput>> + Send + 'static {
    stream! {
        futures_util::pin_mut!(input);
        let mut scanner = SseEventScanner::default();

        while let Some(chunk) = input.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    context
                        .lock()
                        .expect("SSE input context lock poisoned")
                        .pending_error = Some(SseTranslationStreamError::Upstream(error));
                    return;
                }
            };

            for event in scanner.scan(&chunk) {
                if event.is_done_sentinel() {
                    context
                        .lock()
                        .expect("SSE input context lock poisoned")
                        .observe_end(StreamEnd::Done);
                    yield Ok(StreamTranslationInput::End(StreamEnd::Done));
                    return;
                }

                let upstream_event = UpstreamSseEvent {
                    event_type: event.event_type.clone(),
                    data: event.data.clone(),
                };
                context
                    .lock()
                    .expect("SSE input context lock poisoned")
                    .observe_event(upstream_event.clone());

                let data = match event.payload_with_type() {
                    Ok(data) => data,
                    Err(error) => {
                        let failure = StreamTranslationFailure::new(
                            StreamTranslatorErrorStage::Event,
                            stream_input_error(error),
                            Some(upstream_event),
                            None,
                        );
                        context
                            .lock()
                            .expect("SSE input context lock poisoned")
                            .pending_error = Some(failure.into());
                        return;
                    }
                };

                yield Ok(StreamTranslationInput::Event(StreamEvent::new(
                    event.event_type,
                    data,
                )));
            }
        }

        context
            .lock()
            .expect("SSE input context lock poisoned")
            .observe_end(StreamEnd::Eof);
    }
}

fn take_pending_error(context: &Arc<Mutex<SseInputContext>>) -> Option<SseTranslationStreamError> {
    context
        .lock()
        .expect("SSE input context lock poisoned")
        .pending_error
        .take()
}

fn encode_stream_event(event: &StreamEvent) -> serde_json::Result<Bytes> {
    if event.is_done_sentinel() {
        return Ok(done_sentinel_bytes());
    }
    encode_sse_json(&event.event_type, &event.data)
}

fn stream_input_error(error: SseError) -> StreamTranslationError {
    match error {
        SseError::Json(error) => error.into(),
        error => StreamTranslationError::Semantic(error.to_string()),
    }
}

#[cfg(test)]
#[path = "sse_translation_tests.rs"]
mod tests;
