use axum::http::{HeaderMap, HeaderName};

pub(crate) fn filter_forwardable_request_headers(headers: &HeaderMap) -> HeaderMap {
    filter_headers(headers, is_non_forwardable_request_header)
}

pub(crate) fn filter_forwardable_response_headers(headers: &HeaderMap) -> HeaderMap {
    filter_headers(headers, is_non_forwardable_response_header)
}

fn filter_headers(headers: &HeaderMap, should_drop: impl Fn(&str) -> bool) -> HeaderMap {
    let mut forwardable_headers = HeaderMap::new();
    for (key, value) in headers {
        if !should_drop(key.as_str()) {
            forwardable_headers.append(key, value.clone());
        }
    }
    forwardable_headers
}

fn is_non_forwardable_request_header(name: &str) -> bool {
    is_hop_by_hop_header(name)
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "accept-encoding" | "content-length" | "host"
        )
}

fn is_non_forwardable_response_header(name: &str) -> bool {
    is_hop_by_hop_header(name) || name.eq_ignore_ascii_case("content-length")
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

pub(crate) fn is_forwardable_error_response_header(name: &HeaderName) -> bool {
    let name = name.as_str();
    name.eq_ignore_ascii_case("retry-after")
        || name.eq_ignore_ascii_case("x-request-id")
        || name.eq_ignore_ascii_case("request-id")
        || name.starts_with("x-ratelimit-")
        || name.starts_with("anthropic-ratelimit-")
        || name.eq_ignore_ascii_case("openai-processing-ms")
}
