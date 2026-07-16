use serde_json::json;

use super::{normalize_response_payload, normalize_stream_event_payload};

#[test]
fn completes_legacy_response_usage_cache_write_counter() {
    let normalized = normalize_response_payload(json!({
        "usage": {
            "input_tokens_details": {"cached_tokens": 7}
        }
    }));

    assert_eq!(
        normalized["usage"]["input_tokens_details"]["cache_write_tokens"],
        0
    );
}

#[test]
fn completes_legacy_stream_response_usage_cache_write_counter() {
    let normalized = normalize_stream_event_payload(json!({
        "type": "response.completed",
        "response": {
            "usage": {
                "input_tokens_details": {"cached_tokens": 7}
            }
        }
    }));

    assert_eq!(
        normalized["response"]["usage"]["input_tokens_details"]["cache_write_tokens"],
        0
    );
}

#[test]
fn preserves_reported_cache_write_counter() {
    let normalized = normalize_response_payload(json!({
        "usage": {
            "input_tokens_details": {
                "cached_tokens": 7,
                "cache_write_tokens": 3
            }
        }
    }));

    assert_eq!(
        normalized["usage"]["input_tokens_details"]["cache_write_tokens"],
        3
    );
}
