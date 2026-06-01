use crate::config::CaptureConfig;

use super::{CaptureController, CaptureOverrides};
use axum::http::{HeaderMap, Method, Uri};
use std::path::{Path, PathBuf};

#[test]
fn capture_controller_applies_runtime_overrides_over_defaults() {
    let controller = CaptureController::new(
        Some(PathBuf::from("captures")),
        CaptureConfig {
            inbound_request_enabled: false,
            forwarded_request_enabled: false,
            upstream_response_enabled: true,
            outbound_response_enabled: false,
        },
    );

    assert_eq!(
        controller.effective_config(),
        CaptureConfig {
            inbound_request_enabled: false,
            forwarded_request_enabled: false,
            upstream_response_enabled: true,
            outbound_response_enabled: false,
        }
    );

    controller.set_inbound_request_enabled_override(Some(true));
    controller.set_upstream_response_enabled_override(Some(false));

    assert_eq!(
        controller.overrides(),
        CaptureOverrides {
            inbound_request_enabled: Some(true),
            forwarded_request_enabled: None,
            upstream_response_enabled: Some(false),
            outbound_response_enabled: None,
        }
    );
    assert_eq!(
        controller.effective_config(),
        CaptureConfig {
            inbound_request_enabled: true,
            forwarded_request_enabled: false,
            upstream_response_enabled: false,
            outbound_response_enabled: false,
        }
    );

    controller.clear_overrides();
    assert_eq!(controller.overrides(), CaptureOverrides::default());
}

#[tokio::test]
async fn capture_session_records_and_queries_artifacts() {
    let dir = test_capture_dir("records_and_queries");
    let controller = CaptureController::new(
        Some(dir.clone()),
        CaptureConfig {
            inbound_request_enabled: true,
            forwarded_request_enabled: true,
            upstream_response_enabled: false,
            outbound_response_enabled: false,
        },
    );
    let session = controller.session(7);
    let method = Method::POST;
    let uri = Uri::from_static("/v1/responses");
    let headers = HeaderMap::new();

    session
        .capture_inbound_request(&method, &uri, &headers, br#"{"model":"gpt"}"#)
        .await
        .unwrap();
    session
        .capture_forwarded_request(
            &method,
            "https://example.test/v1/responses",
            &headers,
            br#"{"model":"gpt"}"#,
            None,
        )
        .await
        .unwrap();

    let latest = controller.latest_record().unwrap();
    assert_eq!(latest.request_id, 7);
    assert!(latest.inbound_request.is_some());
    assert!(latest.forwarded_request.is_some());
    assert!(latest
        .inbound_request
        .as_ref()
        .unwrap()
        .metadata_path
        .exists());
    assert!(latest
        .forwarded_request
        .as_ref()
        .unwrap()
        .body_path
        .exists());

    let rendered = controller.render_query(&super::CaptureQuery::Show(None));
    assert!(rendered.contains("inbound_request.metadata:"));
    assert!(rendered.contains("forwarded_request.body:"));
}

#[tokio::test]
async fn runtime_override_can_enable_capture_when_defaults_are_disabled() {
    let dir = test_capture_dir("runtime_override");
    let controller = CaptureController::new(Some(dir.clone()), CaptureConfig::default());
    controller.set_inbound_request_enabled_override(Some(true));

    let session = controller.session(8);
    session
        .capture_inbound_request(
            &Method::POST,
            &Uri::from_static("/v1/responses"),
            &HeaderMap::new(),
            br#"{"model":"gpt"}"#,
        )
        .await
        .unwrap();

    let latest = controller.latest_record().unwrap();
    assert_eq!(latest.request_id, 8);
    assert!(latest
        .inbound_request
        .as_ref()
        .unwrap()
        .body_path
        .starts_with(&dir));
}

#[tokio::test]
async fn capture_controller_trims_old_records() {
    let dir = test_capture_dir("trims_old_records");
    let controller = CaptureController::new(
        Some(dir.clone()),
        CaptureConfig {
            inbound_request_enabled: true,
            forwarded_request_enabled: false,
            upstream_response_enabled: false,
            outbound_response_enabled: false,
        },
    );
    let method = Method::POST;
    let uri = Uri::from_static("/v1/responses");
    let headers = HeaderMap::new();

    for request_id in 0..140 {
        controller
            .session(request_id)
            .capture_inbound_request(&method, &uri, &headers, b"{}")
            .await
            .unwrap();
    }

    let records = controller.records();
    assert_eq!(records.len(), 128);
    assert_eq!(records.first().map(|record| record.request_id), Some(12));
    assert_eq!(records.last().map(|record| record.request_id), Some(139));
}

fn test_capture_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "proxai-capture-tests-{name}-{}",
        std::process::id()
    ));
    if Path::new(&path).exists() {
        std::fs::remove_dir_all(&path).unwrap();
    }
    std::fs::create_dir_all(&path).unwrap();
    path
}
