use serde_json::json;

use crate::ingress;
use crate::protocol::ProviderProtocol;
use crate::provider::ProviderRequestView;
use crate::request::RequestId;

use super::translate_request;

fn test_obs() -> crate::observe::ObserveContext {
    let request_id = RequestId::from(1);
    crate::observe::ObserveContext::new(
        request_id,
        std::time::Instant::now(),
        crate::observe::CaptureController::new(None, crate::config::CaptureConfig::default())
            .session(request_id),
        tracing::Span::none(),
    )
}

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

    let obs = test_obs();
    let provider_request = translate_request(
        &inbound,
        ProviderProtocol::OpenaiChatCompletions,
        "MiniMax-M3",
        &obs,
    )
    .expect("translation should produce a Chat Completions provider request");

    let provider_body: serde_json::Value =
        serde_json::from_slice(provider_request.body()).expect("provider body must be JSON");
    assert_eq!(provider_body["model"], "MiniMax-M3");
    assert_eq!(provider_body["max_completion_tokens"], 64);
    assert_eq!(provider_body["stream"], true);
    assert_eq!(provider_body["messages"][0]["role"], "system");
    assert_eq!(provider_body["messages"][0]["content"], "Be concise.");
    assert_eq!(provider_body["messages"][1]["role"], "user");
    assert_eq!(provider_body["messages"][1]["content"][0]["text"], "hello");
    assert_eq!(*provider_request.capture_payload(), provider_body);

    let ProviderRequestView::OpenaiChatCompletions { projection, .. } = provider_request.view()
    else {
        panic!("expected Chat Completions log view");
    };
    assert_eq!(projection.model.as_deref(), Some("MiniMax-M3"));
    assert_eq!(projection.max_completion_tokens, Some(64));
    assert_eq!(projection.stream, Some(true));
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

    let obs = test_obs();
    let provider_request = translate_request(
        &inbound,
        ProviderProtocol::AnthropicMessages,
        "glm-5.1",
        &obs,
    )
    .expect("translation should produce an Anthropic provider request");

    let provider_body: serde_json::Value =
        serde_json::from_slice(provider_request.body()).expect("provider body must be JSON");
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
    assert_eq!(*provider_request.capture_payload(), provider_body);

    let ProviderRequestView::AnthropicMessages { projection, .. } = provider_request.view() else {
        panic!("expected Anthropic log view");
    };
    assert_eq!(projection.model, "glm-5.1");
    assert_eq!(projection.max_tokens, 64);
    assert_eq!(projection.stream, Some(false));
}
