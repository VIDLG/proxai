use async_stream::try_stream;
use axum::body::Bytes;
use futures_util::StreamExt;

use crate::error::ErrorResponseFields;
use crate::http_support::{ByteStream, into_byte_stream};

pub(crate) use crate::sse::encode_sse_json;
use crate::sse::{SseError, SseEvent, SseEventScanner};

pub(crate) type StreamTranslationResult<T> = Result<T, StreamTranslationError>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StreamTranslationError {
    #[error("stream JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error("stream semantic conversion failed: {0}")]
    Semantic(String),

    #[error(transparent)]
    Sse(#[from] SseError),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum StreamTranslatorErrorStage {
    Event,
    Finish,
}

impl StreamTranslatorErrorStage {
    fn default_error_prefix(self) -> &'static str {
        match self {
            Self::Event => "stream translation error",
            Self::Finish => "stream translation finish error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SseStreamEnd {
    DoneSentinel,
    Eof,
}

pub(crate) trait StreamingEventTranslator: Send + 'static {
    fn translate_event(&mut self, event: SseEvent) -> StreamTranslationResult<Vec<Bytes>>;

    fn finish_stream(&mut self, _end: SseStreamEnd) -> StreamTranslationResult<Vec<Bytes>> {
        Ok(Vec::new())
    }
}

pub(crate) fn translate_sse_stream<T>(input: ByteStream, mut translator: T) -> ByteStream
where
    T: StreamingEventTranslator,
{
    let stream = try_stream! {
        futures_util::pin_mut!(input);

        let mut scanner = SseEventScanner::default();

        while let Some(chunk) = input.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    let event = ErrorResponseFields::upstream_response_body_read(format!(
                        "upstream SSE stream error: {error}"
                    ))
                    .encode_sse_event()?;
                    yield event;
                    return;
                }
            };

            for event in scanner.scan(&chunk) {
                if event.is_done_sentinel() {
                    let chunks = match translator.finish_stream(SseStreamEnd::DoneSentinel) {
                        Ok(chunks) => chunks,
                        Err(error) => {
                            let event = ErrorResponseFields::stream_translation(format!(
                                "{}: {error}",
                                StreamTranslatorErrorStage::Finish.default_error_prefix()
                            ))
                            .encode_sse_event()?;
                            yield event;
                            return;
                        }
                    };
                    for translated in chunks {
                        yield translated;
                    }
                    return;
                }

                let chunks = match translator.translate_event(event) {
                    Ok(chunks) => chunks,
                    Err(error) => {
                        let event = ErrorResponseFields::stream_translation(format!(
                            "{}: {error}",
                            StreamTranslatorErrorStage::Event.default_error_prefix()
                        ))
                        .encode_sse_event()?;
                        yield event;
                        return;
                    }
                };
                for translated in chunks {
                    yield translated;
                }
            }
        }

        let chunks = match translator.finish_stream(SseStreamEnd::Eof) {
            Ok(chunks) => chunks,
            Err(error) => {
                let event = ErrorResponseFields::stream_translation(format!(
                    "{}: {error}",
                    StreamTranslatorErrorStage::Finish.default_error_prefix()
                ))
                .encode_sse_event()?;
                yield event;
                return;
            }
        };
        for translated in chunks {
            yield translated;
        }
    };
    into_byte_stream(stream.map(|chunk: serde_json::Result<Bytes>| chunk))
}

#[derive(Debug, Clone)]
pub(crate) struct StreamIdentity {
    id: String,
    model: String,
}

impl StreamIdentity {
    pub(crate) fn new(id: String, model: String) -> Self {
        Self { id, model }
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Debug, Default)]
pub(crate) struct RepresentableOutputTracker {
    emitted_text: bool,
    emitted_refusal: bool,
    emitted_tool_use: bool,
    emitted_reasoning: bool,
}

impl RepresentableOutputTracker {
    pub(crate) fn mark_text(&mut self) {
        self.emitted_text = true;
    }

    pub(crate) fn mark_refusal(&mut self) {
        self.emitted_refusal = true;
    }

    pub(crate) fn mark_tool_use(&mut self) {
        self.emitted_tool_use = true;
    }

    pub(crate) fn mark_reasoning(&mut self) {
        self.emitted_reasoning = true;
    }

    pub(crate) fn emitted_text(&self) -> bool {
        self.emitted_text
    }

    pub(crate) fn emitted_any(&self) -> bool {
        self.emitted_text || self.emitted_refusal || self.emitted_tool_use || self.emitted_reasoning
    }
}
