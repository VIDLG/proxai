use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::{Method, Uri};
use proxai_core::error::JsonPayloadError;
use proxai_core::translation::Translator;
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
    let source = serde_json::from_str::<serde_json::Value>("{\n\n\n    ?\n}").unwrap_err();
    let error = TranslationError::from(JsonPayloadError::new(
        "OpenAI Responses request payload",
        "tools[0]",
        source,
    ));
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

    let _ = fs::remove_dir_all(diagnostics_dir);
}

#[test]
fn stored_payload_uses_json_error_pretty_coordinates() {
    let diagnostics_dir = std::env::temp_dir().join(format!(
        "proxai-translation-diagnostic-coordinates-{}-{}",
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
        "input": [{
            "type": "reasoning",
            "summary": []
        }]
    });
    let error = Translator::new(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::AnthropicMessages,
    )
    .translate_request(&payload)
    .expect_err("id-less reasoning must fail strict translation deserialization");
    let json_error = error
        .as_json_payload_error()
        .expect("translation failure must retain JSON coordinates");
    let point = RequestTranslationFailure {
        method: &method,
        uri: &uri,
        normalized_payload: &payload,
        inbound_request_bytes: 64,
        request_protocol: RequestProtocol::OpenaiResponses,
        provider: "anthropic",
        route_name: Some("responses-to-anthropic"),
        provider_protocol: ProviderProtocol::AnthropicMessages,
        model: "glm-5.2",
        error: &error,
    };

    let bundle = write_request_translation_failure_to_dir(
        RequestId::from(1784160417925),
        &point,
        &diagnostics_dir,
    )
    .unwrap();
    let stored_payload = fs::read_to_string(bundle.join("normalized_payload.json")).unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("record.json")).unwrap()).unwrap();

    assert_eq!(
        stored_payload,
        serde_json::to_string_pretty(&payload).unwrap()
    );
    assert_eq!(record["error"]["line"], json!(json_error.line()));
    assert_eq!(record["error"]["column"], json!(json_error.column()));
    let error_line = stored_payload.lines().nth(json_error.line() - 1).unwrap();
    assert_eq!(error_line.chars().nth(json_error.column() - 1), Some(']'));

    let _ = fs::remove_dir_all(diagnostics_dir);
}
