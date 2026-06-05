use serde_json::json;

use crate::http_support::into_byte_stream;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::TranslationError;

use super::{translate_non_streaming_payload, translate_streaming_stream};

#[test]
fn passes_through_self_protocol_non_streaming_payload() {
    let payload = json!({"error": "upstream failed"});

    let translated = translate_non_streaming_payload(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::OpenaiResponses,
        payload.clone(),
    )
    .unwrap();

    assert_eq!(translated, payload);
}

#[test]
fn rejects_unsupported_success_non_streaming_response_translation() {
    let error = translate_non_streaming_payload(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        json!({"object": "chat.completion"}),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        TranslationError::UnsupportedResponsePair {
            from: ProviderProtocol::OpenaiResponses,
            to: RequestProtocol::OpenaiChatCompletions,
        }
    ));
}

#[test]
fn rejects_unsupported_success_streaming_response_translation() {
    let error = match translate_streaming_stream(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
        into_byte_stream(axum::body::Body::empty().into_data_stream()),
    ) {
        Ok(_) => panic!("expected unsupported streaming response translation error"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        TranslationError::UnsupportedResponsePair {
            from: ProviderProtocol::OpenaiResponses,
            to: RequestProtocol::OpenaiChatCompletions,
        }
    ));
}
