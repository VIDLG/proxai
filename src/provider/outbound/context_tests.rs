use axum::http::HeaderName;

use super::should_forward_error_response_header;

#[test]
fn forwards_only_useful_upstream_error_diagnostic_headers() {
    for name in [
        "retry-after",
        "x-request-id",
        "request-id",
        "x-ratelimit-remaining-requests",
        "anthropic-ratelimit-requests-remaining",
        "openai-processing-ms",
    ] {
        assert!(
            should_forward_error_response_header(&HeaderName::from_static(name)),
            "{name}"
        );
    }

    for name in [
        "authorization",
        "content-length",
        "transfer-encoding",
        "set-cookie",
        "x-api-key",
        "openai-organization",
    ] {
        assert!(
            !should_forward_error_response_header(&HeaderName::from_static(name)),
            "{name}"
        );
    }
}
