use serde_json::json;

use crate::ingress;
use crate::protocol::ProviderProtocol;
use crate::provider::ForwardedRequestView;

use super::translate_request;

#[test]
fn translates_glm_openai_responses_inbound_to_anthropic_forwarded_request() {
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

    let forwarded = translate_request(&inbound, ProviderProtocol::AnthropicMessages, "glm-5.1")
        .expect("translation should produce an Anthropic forwarded request");

    let forwarded_body: serde_json::Value =
        serde_json::from_slice(forwarded.body()).expect("forwarded body must be JSON");
    assert_eq!(forwarded_body["model"], "glm-5.1");
    assert_eq!(forwarded_body["max_tokens"], 64);
    assert_eq!(
        forwarded_body["system"],
        "You are a proxai live translation smoke test. Reply briefly."
    );
    assert_eq!(forwarded_body["stream"], false);
    assert_eq!(forwarded_body["messages"][0]["role"], "user");
    assert_eq!(
        forwarded_body["messages"][0]["content"][0],
        json!({
            "type": "text",
            "text": "Reply with the exact text: proxai-translation-live-ok"
        })
    );
    assert_eq!(*forwarded.capture_payload(), forwarded_body);

    let ForwardedRequestView::AnthropicMessages { projection, .. } = forwarded.view() else {
        panic!("expected Anthropic log view");
    };
    assert_eq!(projection.model, "glm-5.1");
    assert_eq!(projection.max_tokens, 64);
    assert_eq!(projection.stream, Some(false));
}
