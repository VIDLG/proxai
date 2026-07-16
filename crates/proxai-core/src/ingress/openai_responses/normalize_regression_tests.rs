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

// Trigger: Zed replayed Responses reasoning history whose summary item omitted the
// schema-required `id`, as allowed by Zed's `ResponseReasoningInputItem` model.
// Symptom: Responses -> Anthropic request translation failed while deserializing `input`.
// Provenance: diagnostic request 1784160417925 on 2026-07-16; the original 396.1 KB
// payload was reduced to the corresponding item shapes and all prompt content sanitized.
#[test]
fn regression_zed_responses_reasoning_replay_missing_id() {
    let payload = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/regression/zed-responses-reasoning-without-id-request.json"
    ))
    .expect("sanitized regression fixture must be valid JSON");
    let normalized = normalize_payload(payload);

    assert_eq!(normalized["input"][0]["id"], "rs_zed_replay_0");
    assert_eq!(normalized["input"][1]["id"], "rs_existing");
    serde_json::from_value::<crate::protocol::openai_responses::CreateResponseRequest>(
        normalized.clone(),
    )
    .expect("normalized Zed reasoning replay must parse as a Responses request");

    let translator = crate::translation::Translator::new(
        crate::protocol::RequestProtocol::OpenaiResponses,
        crate::protocol::ProviderProtocol::AnthropicMessages,
    );
    let translated = translator
        .translate_request(&normalized)
        .expect("normalized Zed reasoning replay must translate to Anthropic Messages");
    assert_eq!(translated["messages"][0]["role"], "user");
}
