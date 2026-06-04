use axum::body::{Body, to_bytes};
use axum::http::{Response, StatusCode};

use crate::error::InternalError;
use crate::http_support::NonStreamingResponse;
use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{translate_non_streaming_response, translate_streaming_response};

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

#[tokio::test]
async fn rejects_unsupported_success_non_streaming_response_translation() {
    let response = Response::new(Body::from("chat response"));
    let (parts, body) = response.into_parts();
    let body = to_bytes(body, usize::MAX).await.unwrap();

    let error = translate_non_streaming_response(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        NonStreamingResponse::from_parts(parts, body),
    )
    .unwrap_err();

    assert!(matches!(error, InternalError::InvalidRoute(_)));
}

#[test]
fn rejects_unsupported_success_streaming_response_translation() {
    let response = Response::new(Body::empty());

    let error = translate_streaming_response(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        response,
    )
    .unwrap_err();

    assert!(matches!(error, InternalError::InvalidRoute(_)));
}
