use axum::body::Bytes;
use axum::http::Request;

use crate::config::{CaptureConfig, ErrorResponseFormat};
use crate::error::RequestError;
use crate::observe::{CaptureController, ObserveContext};
use crate::protocol::RequestProtocol;
use crate::request::RequestId;

use super::InboundHttpFlow;

#[test]
fn rejects_non_json_payload_before_structured_ingress() {
    let (parts, _) = Request::builder()
        .uri("/v1/responses")
        .body(())
        .unwrap()
        .into_parts();
    let result = InboundHttpFlow::new(
        parts,
        Bytes::from_static(b"not json"),
        test_obs(),
        ErrorResponseFormat::default(),
    )
    .prepare_inbound();

    assert!(matches!(
        result,
        Err(RequestError::InvalidJson {
            protocol: RequestProtocol::OpenaiResponses,
            ..
        })
    ));
}

fn test_obs() -> ObserveContext {
    let request_id = RequestId::from(1);
    ObserveContext::new(
        request_id,
        std::time::Instant::now(),
        CaptureController::new(None, CaptureConfig::default()).session(request_id),
        tracing::Span::none(),
    )
}
