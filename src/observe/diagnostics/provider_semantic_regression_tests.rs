use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use serde_json::Value;

use super::write_anthropic_context_window_exceeded_to_dir;
use crate::http_support::UpstreamResponseHead;
use crate::provider::anthropic_messages::{
    AnthropicResponseState, AnthropicUpstreamResponseSnapshot,
};
use crate::request::RequestId;
use crate::sse::SseEventScanner;
use crate::upstream::UpstreamStreamMetrics;

// Source: a sanitized 2026-07-21 GLM 5.2 response observed during Zed `/compact`.
// The provider exhausted its context before generation and proxai previously
// completed the stream without leaving any diagnostic artifact.
#[test]
fn regression_glm_compaction_context_exhaustion_writes_provider_semantic_diagnostic() {
    let fixture = include_str!(
        "../../../tests/fixtures/regression/anthropic-model-context-window-exceeded-empty-stream.sse"
    );
    let mut scanner = SseEventScanner::default();
    let events = scanner.scan(fixture.as_bytes());
    let mut state = AnthropicResponseState::default();
    state.observe_events(&events);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    let snapshot = AnthropicUpstreamResponseSnapshot {
        head: UpstreamResponseHead::from_headers(
            StatusCode::OK,
            &headers,
            Duration::from_millis(10_632),
        ),
        metrics: UpstreamStreamMetrics::new(
            Duration::from_millis(10_648),
            4,
            fixture.len() as u64,
            Duration::from_millis(10_632),
        ),
        state,
    };
    let diagnostics_dir = std::env::temp_dir().join(format!(
        "proxai-provider-semantic-diagnostic-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos()
    ));

    let bundle = write_anthropic_context_window_exceeded_to_dir(
        RequestId::from(1784191897),
        &snapshot,
        &diagnostics_dir,
    )
    .expect("context exhaustion should create a provider semantic diagnostic");

    let record: Value = serde_json::from_slice(
        &fs::read(bundle.join("record.json")).expect("record should be readable"),
    )
    .expect("record should contain JSON");
    assert_eq!(record["kind"], "provider_semantic_failure");
    assert_eq!(
        record["response"]["stop_reason"],
        "model_context_window_exceeded"
    );
    assert_eq!(record["response"]["model"], "claude-test");
    assert_eq!(record["response"]["input_tokens"], 0);
    assert_eq!(record["response"]["output_tokens"], 0);
    assert_eq!(record["response"]["output_items"], serde_json::json!({}));
    assert_eq!(
        record["artifacts"]["terminal_event"],
        "upstream_terminal_event.sse"
    );

    let terminal_event = fs::read_to_string(bundle.join("upstream_terminal_event.sse"))
        .expect("terminal event should be readable");
    assert!(terminal_event.starts_with("event: message_delta\n"));
    assert!(terminal_event.contains("model_context_window_exceeded"));
    assert!(!terminal_event.contains("compact this conversation"));

    let _ = fs::remove_dir_all(diagnostics_dir);
}
