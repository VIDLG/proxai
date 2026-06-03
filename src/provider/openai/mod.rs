use axum::http::HeaderMap;
use headers::{Authorization, HeaderMapExt, authorization::Bearer};

pub(crate) mod chat_completions;
pub(crate) mod responses;

pub(crate) fn apply_request_auth_headers(headers: &mut HeaderMap, api_key: &str) {
    headers.remove("x-api-key");
    if let Ok(value) = Authorization::<Bearer>::bearer(api_key.trim()) {
        headers.typed_insert(value);
    }
}
