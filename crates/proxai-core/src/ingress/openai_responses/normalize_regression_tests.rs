use serde_json::json;

use super::normalize_payload;

// Trigger: Zed sent a Responses function tool without the schema-required nullable `strict` field.
// Symptom: Responses -> Anthropic request translation failed at `tools[0]` before forwarding.
// Provenance: local Zed request 1784025355355 on 2026-07-14; prompt and tool details sanitized.
#[test]
fn regression_zed_responses_function_tool_missing_strict() {
    let normalized = normalize_payload(json!({
        "model": "glm-5.2",
        "input": "sanitized",
        "tools": [{
            "type": "function",
            "name": "lookup",
            "description": "Sanitized tool",
            "parameters": {"type": "object", "properties": {}}
        }]
    }));

    assert_eq!(normalized["tools"][0]["strict"], serde_json::Value::Null);
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(
        normalized.clone(),
    )
    .expect("normalized Zed function tool must parse as a Responses request");

    let translator = crate::translation::Translator::new(
        crate::protocol::RequestProtocol::OpenaiResponses,
        crate::protocol::ProviderProtocol::AnthropicMessages,
    );
    let translated = translator
        .translate_request(&normalized)
        .expect("normalized Zed function tool must translate to Anthropic Messages");
    assert_eq!(translated["tools"][0]["name"], "lookup");
}
