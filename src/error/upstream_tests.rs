use serde_json::json;

use super::UpstreamResponseError;
use crate::protocol::ProviderProtocol;

#[test]
fn parses_openai_error_shape_and_displays_code_and_param() {
    let body = serde_json::to_vec(&json!({
        "error": {
            "code": "array_above_max_length",
            "message": "Invalid 'input[3].content': array too long.",
            "param": "input[3].content",
            "type": "invalid_request_error"
        }
    }))
    .unwrap();

    let error = UpstreamResponseError::parse_body(ProviderProtocol::OpenaiResponses, &body);

    assert_eq!(error.upstream_code(), Some("array_above_max_length"));
    assert_eq!(
        error.upstream_message(),
        Some("Invalid 'input[3].content': array too long.")
    );
    assert_eq!(error.upstream_param(), Some(&json!("input[3].content")));
    assert_eq!(
        error.to_string(),
        "upstream response error: Invalid 'input[3].content': array too long. code=array_above_max_length param=input[3].content"
    );
}

#[test]
fn keeps_carrier_failures_in_the_application_error() {
    assert!(matches!(
        UpstreamResponseError::parse_body(ProviderProtocol::OpenaiResponses, b"  "),
        UpstreamResponseError::EmptyBody
    ));
    assert!(matches!(
        UpstreamResponseError::parse_body(ProviderProtocol::OpenaiResponses, b"not json"),
        UpstreamResponseError::NonJsonBody { .. }
    ));
    assert!(matches!(
        UpstreamResponseError::parse_body(
            ProviderProtocol::OpenaiResponses,
            br#"{"unexpected":true}"#,
        ),
        UpstreamResponseError::UnknownBodyShape { .. }
    ));
}
