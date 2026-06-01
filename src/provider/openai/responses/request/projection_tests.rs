use super::super::RequestSummary;
use super::{adapt_payload_for_projection, project_payload};
use serde_json::json;

#[test]
fn request_adaptation_defaults_missing_input_image_detail() {
    let payload = json!({
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_image",
                        "image_url": "data:image/png;base64,AAAA"
                    }
                ]
            }
        ]
    });

    let adapted = adapt_payload_for_projection(&payload);
    assert_eq!(
        adapted
            .pointer("/input/0/content/0/detail")
            .and_then(|value| value.as_str()),
        Some("auto")
    );
}

#[test]
fn project_payload_supports_request_summary_extraction() {
    let payload = json!({
        "model": "gpt-5.5",
        "tools": [
            {
                "type": "function",
                "name": "edit_file"
            },
            {
                "type": "web_search_preview"
            }
        ],
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "hello"
                    }
                ]
            }
        ]
    });

    let projection = project_payload(&payload, None).expect("project request payload");
    let summary = RequestSummary::from(&projection);

    assert_eq!(summary.tool_inventory.len(), 2);
}
