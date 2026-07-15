use serde_json::json;

use crate::ingress::prepare_inbound_request;
use crate::protocol::{ProviderProtocol, RequestProtocol};
use crate::translation::Translator;

#[test]
fn translates_openai_responses_inbound_to_chat_provider_request() {
    let inbound = prepare_inbound_request(
        RequestProtocol::OpenaiResponses,
        json!({
            "model": "glm-5.1",
            "instructions": "Be concise.",
            "input": [{
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello"}]
            }],
            "stream": true,
            "max_output_tokens": 64
        }),
    )
    .unwrap();

    let translator = Translator::new(inbound.protocol(), ProviderProtocol::OpenaiChatCompletions);
    let provider_body = translator
        .translate_request(inbound.normalized_payload())
        .expect("translation should produce a Chat Completions payload");

    assert_eq!(provider_body["model"], "glm-5.1");
    assert_eq!(provider_body["max_completion_tokens"], 64);
    assert_eq!(provider_body["stream"], true);
    assert_eq!(provider_body["messages"][0]["role"], "developer");
    assert_eq!(provider_body["messages"][0]["content"], "Be concise.");
    assert_eq!(provider_body["messages"][1]["role"], "user");
    assert_eq!(provider_body["messages"][1]["content"][0]["text"], "hello");
}

#[test]
fn translates_glm_openai_responses_inbound_to_anthropic_provider_request() {
    let inbound = prepare_inbound_request(
        RequestProtocol::OpenaiResponses,
        json!({
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
        }),
    )
    .unwrap();

    let translator = Translator::new(inbound.protocol(), ProviderProtocol::AnthropicMessages);
    let provider_body = translator
        .translate_request(inbound.normalized_payload())
        .expect("translation should produce an Anthropic payload");

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
