use axum::http::{HeaderMap, HeaderValue};

mod prepare;
mod summary;

pub(crate) const UPSTREAM_PATH: &str = "/v1/messages";

pub(crate) use self::prepare::{PreparedProviderRequest, prepare_provider_request};
pub(crate) use self::summary::{RequestSummary, ToolCategory};

pub(crate) fn apply_auth_headers(headers: &mut HeaderMap, api_key: &str) {
    headers.remove(http::header::AUTHORIZATION);
    if let Ok(value) = HeaderValue::from_str(api_key.trim()) {
        headers.insert("x-api-key", value);
    }
}

#[cfg(test)]
mod tests;
