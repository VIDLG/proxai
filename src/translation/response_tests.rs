use axum::body::{Body, to_bytes};
use axum::http::{Response, StatusCode};

use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::translate_non_streaming_response;

#[tokio::test]
async fn skips_translation_for_upstream_error_responses() {
    let mut response = Response::new(Body::from("upstream failed"));
    *response.status_mut() = StatusCode::BAD_GATEWAY;

    let response = translate_non_streaming_response(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::AnthropicMessages,
        response,
    )
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"upstream failed");
}
