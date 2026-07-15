use super::prepare_openai_responses_request;
use crate::ingress::IngressError;
use crate::protocol::RequestProtocol;
use serde_json::json;

#[test]
fn prepare_request_normalizes_payload_and_extracts_model() {
    let request = json!({
        "model": "gpt-5.5",
        "instructions": "existing",
        "input": [
            {
                "role": "system",
                "content": [{ "type": "input_text", "text": "be concise" }]
            },
            {
                "role": "user",
                "content": [{ "type": "input_text", "text": "hello" }]
            }
        ]
    });

    let prepared = prepare_openai_responses_request(request).unwrap();

    assert_eq!(prepared.model(), "gpt-5.5");
    assert_eq!(
        prepared.normalized_payload(),
        &json!({
            "model": "gpt-5.5",
            "instructions": "be concise\n\nexisting",
            "input": [
                {
                    "role": "user",
                    "content": [{ "type": "input_text", "text": "hello" }]
                }
            ]
        })
    );
}

#[test]
fn prepare_request_rejects_missing_or_empty_model_values() {
    let missing = json!({});
    let empty = json!({ "model": "   " });

    for payload in [missing, empty] {
        assert!(matches!(
            prepare_openai_responses_request(payload),
            Err(IngressError::MissingModel {
                protocol: RequestProtocol::OpenaiResponses,
            })
        ));
    }
}
