use axum::body::{Body, to_bytes};
use axum::http::{Response, StatusCode};

use crate::http_support::NonStreamingResponse;
use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::translate_non_streaming_response;

#[tokio::test]
async fn skips_translation_for_upstream_error_responses() {
    let mut response = Response::new(Body::from("upstream failed"));
    *response.status_mut() = StatusCode::BAD_GATEWAY;

    let (parts, body) = response.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap();
    let response = translate_non_streaming_response(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::AnthropicMessages,
        NonStreamingResponse::from_parts(parts, body),
    )
    .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"upstream failed");
}
