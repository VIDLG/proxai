use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::{Method, Uri};
use serde_json::json;

use crate::observe::point::RequestTranslationFailure;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::request::RequestId;
use crate::translation::TranslationError;

use super::write_request_translation_failure_to_dir;

#[test]
fn stores_normalized_payload_and_json_location_without_capture() {
    let diagnostics_dir = std::env::temp_dir().join(format!(
        "proxai-translation-diagnostic-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let method = Method::POST;
    let uri = Uri::from_static("/v1/responses");
    let payload = json!({
        "model": "glm-5.2",
        "tools": [{"type": "function", "name": "lookup"}]
    });
    let error = TranslationError::JsonPayload {
        context: "OpenAI Responses request payload",
        path: "tools[0]".to_string(),
        message: "missing field `strict`".to_string(),
        line: 4,
        column: 5,
    };
    let point = RequestTranslationFailure {
        method: &method,
        uri: &uri,
        normalized_payload: &payload,
        inbound_request_bytes: 128,
        request_protocol: RequestProtocol::OpenaiResponses,
        provider: "anthropic",
        route_name: Some("responses-to-anthropic"),
        provider_protocol: ProviderProtocol::AnthropicMessages,
        model: "glm-5.2",
        error: &error,
    };

    let bundle = write_request_translation_failure_to_dir(
        RequestId::from(1784025355355),
        &point,
        &diagnostics_dir,
    )
    .unwrap();
    let stored_payload: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("normalized_payload.json")).unwrap()).unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("record.json")).unwrap()).unwrap();

    assert_eq!(stored_payload, payload);
    assert_eq!(record["error"]["json_path"], "tools[0]");
    assert_eq!(record["error"]["line"], 4);
    assert_eq!(record["error"]["column"], 5);
    assert_eq!(record["route"]["provider_protocol"], "anthropic_messages");

    fs::remove_dir_all(diagnostics_dir).unwrap();
}
