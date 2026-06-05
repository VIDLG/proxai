use async_stream::try_stream;
use axum::body::Bytes;
use futures_util::StreamExt;

use crate::error::ErrorResponseFields;
use crate::http_support::{ByteStream, into_byte_stream};

pub(crate) use crate::sse::encode_sse_json;
use crate::sse::{SseError, SseEvent, SseEventScanner};

pub(crate) type SseTranslationResult<T> = Result<T, SseTranslationError>;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SseTranslationError {
    #[error("SSE JSON conversion failed: {0}")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Sse(#[from] SseError),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SseTranslatorErrorStage {
    Event,
    Finish,
}

impl SseTranslatorErrorStage {
    fn default_error_prefix(self) -> &'static str {
        match self {
            Self::Event => "SSE translation error",
            Self::Finish => "SSE translation finish error",
        }
    }
}

pub(crate) trait SseEventTranslator: Send + 'static {
    fn translate_event(&mut self, event: SseEvent) -> SseTranslationResult<Vec<Bytes>>;

    fn finish(&mut self) -> SseTranslationResult<Vec<Bytes>> {
        Ok(Vec::new())
    }
}

pub(crate) fn translate_sse_stream<T>(input: ByteStream, mut translator: T) -> ByteStream
where
    T: SseEventTranslator,
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
                let chunks = match translator.translate_event(event) {
                    Ok(chunks) => chunks,
                    Err(error) => {
                        let event = ErrorResponseFields::sse_translation(format!(
                            "{}: {error}",
                            SseTranslatorErrorStage::Event.default_error_prefix()
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

        let chunks = match translator.finish() {
            Ok(chunks) => chunks,
            Err(error) => {
                let event = ErrorResponseFields::sse_translation(format!(
                    "{}: {error}",
                    SseTranslatorErrorStage::Finish.default_error_prefix()
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
