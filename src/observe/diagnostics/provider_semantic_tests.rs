use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use serde_json::Value;

use super::write_openai_responses_error_to_dir;
use crate::http_support::UpstreamResponseHead;
use crate::observe::point::{
    ProviderSemanticFailure, ProviderStreamOutcome, ProviderStreamOutcomeObserved,
    ProviderStreamSnapshot,
};
use crate::provider::openai::responses::{ResponsesUpstreamState, ResponsesUpstreamStreamSnapshot};
use crate::request::RequestId;
use crate::sse::SseEventScanner;
use crate::upstream::UpstreamStreamMetrics;

#[test]
fn writes_openai_responses_terminal_error_diagnostic() {
    let stream = concat!(
        "event: error\n",
        "data: {\"type\":\"error\",\"sequence_number\":4,\"error\":{\"type\":\"invalid_request_error\",\"code\":\"context_length_exceeded\",\"message\":\"Your input exceeds the context window of this model.\",\"param\":\"input\"}}\n\n"
    );
    let mut scanner = SseEventScanner::default();
    let events = scanner.scan(stream.as_bytes());
    let mut state = ResponsesUpstreamState::default();
    state.observe_events(&events);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    let snapshot = ResponsesUpstreamStreamSnapshot {
        head: UpstreamResponseHead::from_headers(
            StatusCode::OK,
            &headers,
            Duration::from_millis(5_964),
        ),
        metrics: UpstreamStreamMetrics::new(
            Duration::from_millis(49_017),
            90,
            146_800,
            Duration::from_millis(19_996),
        ),
        state,
        recent_tail: stream.as_bytes().to_vec(),
        metadata: Default::default(),
    };
    let point = ProviderStreamOutcomeObserved {
        snapshot: ProviderStreamSnapshot::OpenaiResponses(&snapshot),
        outcome: ProviderStreamOutcome::Completed,
    };
    assert_eq!(
        point.semantic_failure(),
        Some(ProviderSemanticFailure::ContextWindowExceeded)
    );

    let generic_error_stream = concat!(
        "event: error\n",
        "data: {\"type\":\"error\",\"sequence_number\":5,\"error\":{\"type\":\"server_error\",\"code\":\"server_error\",\"message\":\"The provider could not complete the request.\",\"param\":null}}\n\n"
    );
    let generic_events = scanner.scan(generic_error_stream.as_bytes());
    let mut generic_error_state = ResponsesUpstreamState::default();
    generic_error_state.observe_events(&generic_events);
    let mut generic_error_snapshot = snapshot.clone();
    generic_error_snapshot.state = generic_error_state;
    let generic_error_point = ProviderStreamOutcomeObserved {
        snapshot: ProviderStreamSnapshot::OpenaiResponses(&generic_error_snapshot),
        outcome: ProviderStreamOutcome::Completed,
    };
    assert_eq!(
        generic_error_point.semantic_failure(),
        Some(ProviderSemanticFailure::ProviderReportedError)
    );

    let diagnostics_dir = std::env::temp_dir().join(format!(
        "proxai-openai-provider-semantic-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos()
    ));
    let bundle = write_openai_responses_error_to_dir(
        RequestId::from(1784601174),
        &snapshot,
        &diagnostics_dir,
    )
    .expect("Responses terminal error should create a diagnostic bundle");

    let record: Value = serde_json::from_slice(
        &fs::read(bundle.join("record.json")).expect("record should be readable"),
    )
    .expect("record should contain JSON");
    assert_eq!(record["kind"], "provider_semantic_failure");
    assert_eq!(record["provider_protocol"], "openai_responses");
    assert_eq!(
        record["response"]["error"]["code"],
        "context_length_exceeded"
    );
    assert_eq!(record["response"]["sequence_number"], 4);
    assert_eq!(
        record["artifacts"]["terminal_event"],
        "upstream_terminal_event.sse"
    );

    let terminal_event = fs::read_to_string(bundle.join("upstream_terminal_event.sse"))
        .expect("terminal event should be readable");
    assert_eq!(terminal_event, stream);
    assert!(!terminal_event.contains("Authorization"));

    let _ = fs::remove_dir_all(diagnostics_dir);
}
