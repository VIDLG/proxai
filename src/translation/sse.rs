use async_stream::try_stream;
use axum::body::{Body, Bytes};
use axum::http::{Response, header};
use futures_util::{Stream, StreamExt};

use crate::error::ErrorResponseFields;

pub(crate) use crate::sse::encode_sse_json;
use crate::sse::{SseEvent, SseEventScanner};
use std::io;

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
    fn translate_event(&mut self, event: SseEvent) -> io::Result<Vec<Bytes>>;

    fn finish(&mut self) -> io::Result<Vec<Bytes>> {
        Ok(Vec::new())
    }
}

pub(crate) fn translate_sse_response<T>(response: Response<Body>, translator: T) -> Response<Body>
where
    T: SseEventTranslator,
{
    let (mut parts, body) = response.into_parts();
    parts.headers.remove(header::CONTENT_LENGTH);
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/event-stream"),
    );

    let stream = translate_sse_body(body, translator);
    Response::from_parts(parts, Body::from_stream(stream))
}

fn translate_sse_body<T>(
    body: Body,
    mut translator: T,
) -> impl Stream<Item = io::Result<Bytes>> + Send + 'static
where
    T: SseEventTranslator,
{
    try_stream! {
        let mut input = body.into_data_stream();
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
    }
}
