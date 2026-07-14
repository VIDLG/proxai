use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::{Method, Uri};
use serde_json::Value;

use crate::observe::point::StreamingTranslationFailure;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::request::RequestId;
use crate::translation::streaming::{
    StreamTranslationFailure, StreamTranslatorErrorStage, UpstreamSseEvent,
};

use super::write_streaming_translation_failure_to_dir;

#[test]
fn stores_raw_triggering_sse_event_without_capture() {
    let diagnostics_dir = std::env::temp_dir().join(format!(
        "proxai-stream-diagnostic-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let method = Method::POST;
    let uri: Uri = "/v1/responses".parse().unwrap();
    let failure = StreamTranslationFailure {
        stage: StreamTranslatorErrorStage::Event,
        error:
            "stream translation error: stream JSON conversion failed: missing field `finish_reason`"
                .to_string(),
        upstream_event: Some(UpstreamSseEvent {
            event_type: "message".to_string(),
            data: "{\"choices\":[{\"delta\":{}}]}".to_string(),
        }),
        end: None,
    };
    let point = StreamingTranslationFailure {
        method: &method,
        uri: &uri,
        request_protocol: RequestProtocol::OpenaiResponses,
        provider_protocol: ProviderProtocol::OpenaiChatCompletions,
        failure: &failure,
    };

    let bundle = write_streaming_translation_failure_to_dir(
        RequestId::from(1784028575780),
        &point,
        &diagnostics_dir,
    )
    .expect("stream translation failure should create a diagnostic bundle");

    let record: Value = serde_json::from_slice(&fs::read(bundle.join("record.json")).unwrap())
        .expect("record should be JSON");
    assert_eq!(record["kind"], "stream_translation_failure");
    assert_eq!(record["failure"]["stage"], "event");
    assert_eq!(record["failure"]["upstream_event_type"], "message");
    assert_eq!(
        record["artifacts"]["upstream_sse_event"],
        "upstream_sse_event.sse"
    );
    assert_eq!(
        fs::read_to_string(bundle.join("upstream_sse_event.sse")).unwrap(),
        "data: {\"choices\":[{\"delta\":{}}]}\n\n"
    );

    fs::remove_dir_all(diagnostics_dir).unwrap();
}
