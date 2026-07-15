use std::sync::Arc;

use futures_util::{StreamExt, stream};
use serde_json::{Value, json};

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::context::TranslationContext;
use super::stream::{StreamEnd, StreamEvent, StreamTranslationInput};
use super::{
    NoopTranslationObserver, TranslationPhase, TranslationRoute, TranslationScope, Translator,
};

pub(crate) fn request_scope(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
) -> TranslationScope {
    translation_scope(
        request_protocol,
        provider_protocol,
        TranslationPhase::Request,
    )
}

pub(crate) fn response_scope(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
) -> TranslationScope {
    translation_scope(
        request_protocol,
        provider_protocol,
        TranslationPhase::NonStreamingResponse,
    )
}

fn translation_scope(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
    phase: TranslationPhase,
) -> TranslationScope {
    TranslationContext::new(
        TranslationRoute {
            request_protocol,
            provider_protocol,
        },
        Arc::new(NoopTranslationObserver),
    )
    .scope(phase)
}

pub(crate) async fn translate_sse_fixture(fixture: &str, translator: Translator) -> String {
    let input = stream::iter(parse_sse_fixture(fixture).into_iter().map(Ok));
    let output = translator.translate_stream(input).collect::<Vec<_>>().await;

    let mut rendered = String::new();
    for item in output {
        match item {
            Ok(event) => render_event(&mut rendered, event),
            Err(error) => {
                render_event(
                    &mut rendered,
                    StreamEvent::new(
                        "error",
                        json!({"error": {"message": format!("stream translation error: {error}")}}),
                    ),
                );
                break;
            }
        }
    }
    rendered
}

pub(crate) fn parse_rendered_events(rendered: &str) -> Vec<StreamEvent> {
    parse_sse_fixture(rendered)
        .into_iter()
        .filter_map(|input| match input {
            StreamTranslationInput::Event(event) => Some(event),
            StreamTranslationInput::End(_) => None,
        })
        .collect()
}

fn parse_sse_fixture(fixture: &str) -> Vec<StreamTranslationInput> {
    fixture
        .replace("\r\n", "\n")
        .split("\n\n")
        .filter_map(parse_frame)
        .collect()
}

fn parse_frame(frame: &str) -> Option<StreamTranslationInput> {
    if frame.trim().is_empty() {
        return None;
    }

    let mut event_type = "message";
    let mut data = Vec::new();
    for line in frame.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event_type = value.trim();
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(value.trim_start());
        }
    }

    let data = data.join("\n");
    if data == "[DONE]" {
        return Some(StreamTranslationInput::End(StreamEnd::Done));
    }

    let mut payload: Value = serde_json::from_str(&data)
        .unwrap_or_else(|error| panic!("invalid SSE fixture JSON `{data}`: {error}"));
    if event_type != "message"
        && let Some(object) = payload.as_object_mut()
    {
        object
            .entry("type".to_string())
            .or_insert_with(|| Value::String(event_type.to_string()));
    }

    Some(StreamTranslationInput::Event(StreamEvent::new(
        event_type, payload,
    )))
}

fn render_event(rendered: &mut String, event: StreamEvent) {
    if event.is_done_sentinel() {
        rendered.push_str("data: [DONE]\n\n");
        return;
    }

    rendered.push_str("event: ");
    rendered.push_str(&event.event_type);
    rendered.push_str("\ndata: ");
    rendered.push_str(&serde_json::to_string(&event.data).expect("stream event must serialize"));
    rendered.push_str("\n\n");
}
