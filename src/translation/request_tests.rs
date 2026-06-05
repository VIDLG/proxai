use serde_json::json;

use crate::ingress;
use crate::protocol::ProviderProtocol;

use super::translate_request;

#[test]
fn translates_openai_responses_inbound_to_chat_provider_request() {
    let inbound_body = serde_json::to_vec(&json!({
        "model": "glm-5.1",
        "instructions": "Be concise.",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "hello"}]
        }],
        "stream": true,
        "max_output_tokens": 64
    }))
    .unwrap();
    let inbound = ingress::openai_responses::prepare_openai_responses_request(&inbound_body)
        .map(ingress::PreparedInboundRequest::OpenaiResponses)
        .unwrap();

    let translated = translate_request(
        inbound.protocol(),
        ProviderProtocol::OpenaiChatCompletions,
        inbound.normalized_payload(),
    )
    .expect("translation should produce a Chat Completions payload");

    let provider_body = translated;
    assert_eq!(provider_body["model"], "glm-5.1");
    assert_eq!(provider_body["max_completion_tokens"], 64);
    assert_eq!(provider_body["stream"], true);
    assert_eq!(provider_body["messages"][0]["role"], "system");
    assert_eq!(provider_body["messages"][0]["content"], "Be concise.");
    assert_eq!(provider_body["messages"][1]["role"], "user");
    assert_eq!(provider_body["messages"][1]["content"][0]["text"], "hello");
}
#[test]
fn translates_glm_openai_responses_inbound_to_anthropic_provider_request() {
    let inbound_body = serde_json::to_vec(&json!({
        "model": "glm-5.1",
        "instructions": "You are a proxai live translation smoke test. Reply briefly.",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "Reply with the exact text: proxai-translation-live-ok"
            }]
        }],
        "stream": false,
        "max_output_tokens": 64
    }))
    .unwrap();
    let inbound = ingress::openai_responses::prepare_openai_responses_request(&inbound_body)
        .map(ingress::PreparedInboundRequest::OpenaiResponses)
        .unwrap();

    let translated = translate_request(
        inbound.protocol(),
        ProviderProtocol::AnthropicMessages,
        inbound.normalized_payload(),
    )
    .expect("translation should produce an Anthropic payload");

    let provider_body = translated;
    assert_eq!(provider_body["model"], "glm-5.1");
    assert_eq!(provider_body["max_tokens"], 64);
    assert_eq!(
        provider_body["system"],
        "You are a proxai live translation smoke test. Reply briefly."
    );
    assert_eq!(provider_body["stream"], false);
    assert_eq!(provider_body["messages"][0]["role"], "user");
    assert_eq!(
        provider_body["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Reply with the exact text: proxai-translation-live-ok"
        })
    );
}
