use super::prepare_openai_responses_request;
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

    let prepared = prepare_openai_responses_request(request.to_string().as_bytes()).unwrap();

    assert_eq!(prepared.model, "gpt-5.5");
    assert_eq!(
        prepared.normalized_payload,
        json!({
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
fn prepare_request_rejects_non_json_payloads() {
    let error = prepare_openai_responses_request(b"not json").unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid request: OpenAI Responses requests must be JSON and include a non-empty `model`."
    );
}

#[test]
fn prepare_request_rejects_missing_or_empty_model_values() {
    let missing = json!({});
    let empty = json!({ "model": "   " });

    assert!(prepare_openai_responses_request(missing.to_string().as_bytes()).is_err());
    assert!(prepare_openai_responses_request(empty.to_string().as_bytes()).is_err());
}
