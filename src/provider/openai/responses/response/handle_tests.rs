use serde::Deserialize;
use serde_json::Value;

use super::{OpenaiResponsesUpstreamBodyObserver, error_sse_chunk};
use crate::request::RequestId;
use crate::upstream::BodyObserver;

fn test_obs() -> crate::observe::ObserveContext {
    let request_id = RequestId::from(1);
    crate::observe::ObserveContext::new(
        request_id,
        std::time::Instant::now(),
        crate::observe::CaptureController::new(None, crate::config::CaptureConfig::default())
            .session(request_id),
        tracing::Span::none(),
    )
}

fn test_observer() -> OpenaiResponsesUpstreamBodyObserver {
    OpenaiResponsesUpstreamBodyObserver::new(None, test_obs())
}

#[test]
fn sse_eof_without_terminal_event_is_incomplete_even_without_pending_tools() {
    let mut observer = test_observer();

    observer.on_chunk(
        br#"data: {"type":"response.output_text.delta","sequence_number":1,"delta":"ok"}

"#,
    );

    assert!(!observer.saw_terminal_event);
    assert!(observer.stream_error.is_none());
}

#[test]
fn sse_eof_after_terminal_event_is_complete() {
    let mut observer = test_observer();

    observer.on_chunk(
        br#"data: {"type":"response.completed","sequence_number":2}

"#,
    );

    assert!(observer.saw_terminal_event);
    assert!(observer.stream_error.is_none());
}

#[test]
fn injected_stream_error_matches_zed_responses_top_level_error_shape() {
    let frame = error_sse_chunk(Some(7), "tool stream stalled");
    let data = sse_data(&frame);

    let event = serde_json::from_str::<ZedResponsesStreamEvent>(&data).unwrap();
    let ZedResponsesStreamEvent::GenericError { error } = event else {
        panic!("expected generic Responses error event");
    };

    let error = error.into_response_error();
    assert_eq!(error.message, "tool stream stalled");
    assert_eq!(error.code, None);
    assert_eq!(error.param, None);
}

fn sse_data(frame: &[u8]) -> String {
    std::str::from_utf8(frame)
        .unwrap()
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ZedResponsesStreamEvent {
    #[serde(rename = "error")]
    GenericError {
        #[serde(flatten)]
        error: ZedGenericStreamErrorPayload,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug, Default)]
struct ZedGenericStreamErrorPayload {
    #[serde(flatten)]
    top_level: PartialZedResponseError,
    #[serde(default)]
    error: Option<PartialZedResponseError>,
}

#[derive(Deserialize, Debug, Default)]
struct PartialZedResponseError {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    param: Option<Value>,
}

impl ZedGenericStreamErrorPayload {
    fn into_response_error(self) -> ZedResponseError {
        let nested = self.error.unwrap_or_default();
        ZedResponseError {
            code: self.top_level.code.or(nested.code),
            message: self
                .top_level
                .message
                .or(nested.message)
                .unwrap_or_default(),
            param: self.top_level.param.or(nested.param),
        }
    }
}

#[derive(Debug)]
struct ZedResponseError {
    code: Option<String>,
    message: String,
    param: Option<Value>,
}

#[test]
fn tool_argument_delta_without_item_id_marks_stream_error() {
    let mut observer = test_observer();

    observer.on_chunk(
        br#"data: {"type":"response.function_call_arguments.delta","sequence_number":1,"delta":"{}"}

"#,
    );

    assert!(observer.saw_terminal_event);
    assert!(observer.stream_error.is_some());
}
