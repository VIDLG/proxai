use serde_json::Value;

use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::openai_responses::to_anthropic_messages::translate_request_payload;
use crate::translation::test_support::request_scope;

// Trigger: Zed sent a valid Responses request with reasoning.effort="none" through
// a glm-5.2 Responses -> Anthropic route; translation failed before forwarding by
// forcing `none` into Anthropic output_config.effort. Provenance: diagnostic
// 1784191829-1784191829910 (2026-07-16); the 280.1 KB payload was reduced and its
// private prompt and conversation content were removed.
#[test]
fn regression_zed_reasoning_none_rejected_before_anthropic_forward() {
    let scope = request_scope(
        RequestProtocol::OpenaiResponses,
        ProviderProtocol::AnthropicMessages,
    );
    let request: Value = serde_json::from_str(include_str!(
        "../../../../../../../tests/fixtures/regression/zed-responses-reasoning-none-request.json"
    ))
    .unwrap();
    let translated = translate_request_payload(&request, &scope).unwrap();

    assert!(translated.get("output_config").is_none());
    assert_eq!(translated["thinking"]["type"], "disabled");
}
