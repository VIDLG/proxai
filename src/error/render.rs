use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};
use delegate::delegate;
use serde::Serialize;
use serde_json::Value;

use crate::config::ErrorResponseFormat;
use crate::http_support::{is_forwardable_error_response_header, response_with_headers};
use crate::sse::encode_sse_json;

use super::{Error, RequestError, UpstreamError, UpstreamResponseError};

/// Client-facing error payload embedded in JSON HTTP responses and SSE error events.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErrorResponsePayload {
    message: String,
    #[serde(rename = "type")]
    error_type: ErrorResponseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    param: Option<Value>,
    // Numeric status included in JSON/SSE payloads. This is derived from
    // `ErrorResponseFields::http_status`; SSE streams cannot change the HTTP
    // status line after they have started.
    status: u16,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorResponseType {
    InvalidRequestError,
    InternalError,
    UpstreamRequestError,
    UpstreamError,
    UpstreamResponseBodyReadError,
    UpstreamErrorBodyEmpty,
    UpstreamErrorBodyNonJson,
    UpstreamErrorBodyUnknownShape,
    StreamTranslationError,
}

#[derive(Serialize)]
struct ErrorJsonResponse {
    error: ErrorResponsePayload,
}

// Generic SSE error envelope emitted by proxai for stream translation/transport
// failures. `event: error` plus data `type: "error"` identifies the event as an
// error for OpenAI-compatible SSE clients.
//
// This is not a Chat Completions stream shape. Zed's Chat Completions parser in
// `contrib/zed/crates/open_ai/src/open_ai.rs` reads only `data:` lines and
// accepts either a normal chat chunk or `{ "error": { "message": ... } }`.
//
// For Responses streams, Zed v1.5.3 parses `type: "error"` as
// `GenericStreamErrorPayload`, accepting both top-level `message`/`code`/`param`
// and this nested `error` object. Its standard Responses error event remains
// `type: "response.error"` with a nested `error` object.
#[derive(Serialize)]
struct ErrorSseEvent {
    #[serde(rename = "type")]
    event_type: &'static str,
    error: ErrorResponsePayload,
}

impl ErrorResponsePayload {
    fn encode_sse_event(self) -> serde_json::Result<Bytes> {
        encode_sse_json(
            "error",
            &ErrorSseEvent {
                event_type: "error",
                error: self,
            },
        )
    }

    fn encode_sse_event_or_fallback(self) -> Bytes {
        self.encode_sse_event().unwrap_or_else(|_| {
            // Last-resort literal used only if typed SSE serialization fails.
            // Keep this allocation-free and independent of serde so the fallback
            // cannot recursively fail while closing an already-broken stream.
            Bytes::from_static(FALLBACK_SSE_ERROR_EVENT)
        })
    }
}

/// Client-facing error projection derived from proxai's typed error model.
///
/// This is intentionally separate from `Error`/`RequestError`/`UpstreamError`:
/// those enums model internal failure causes, while this type models the stable
/// HTTP/SSE shape exposed to clients.
///
/// `http_status` is kept as an HTTP `StatusCode` for response rendering, while
/// `payload` is the wire payload reused for JSON and SSE error shapes.
#[derive(Debug, Clone)]
pub(crate) struct ErrorResponseFields {
    http_status: StatusCode,
    payload: ErrorResponsePayload,
}

const FALLBACK_SSE_ERROR_EVENT: &[u8] = b"event: error\ndata: {\"type\":\"error\",\"error\":{\"message\":\"stream error\",\"type\":\"internal_error\",\"status\":500}}\n\n";

impl ErrorResponseFields {
    fn new(
        http_status: StatusCode,
        message: impl Into<String>,
        error_type: ErrorResponseType,
    ) -> Self {
        Self::new_with_details(http_status, message, error_type, None, None)
    }

    fn new_with_details(
        http_status: StatusCode,
        message: impl Into<String>,
        error_type: ErrorResponseType,
        code: Option<String>,
        param: Option<Value>,
    ) -> Self {
        Self {
            http_status,
            payload: ErrorResponsePayload {
                message: message.into(),
                error_type,
                code,
                param,
                status: http_status.as_u16(),
            },
        }
    }

    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            message,
            ErrorResponseType::InvalidRequestError,
        )
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            message,
            ErrorResponseType::InternalError,
        )
    }

    pub(crate) fn upstream_request(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            message,
            ErrorResponseType::UpstreamRequestError,
        )
    }

    pub(crate) fn upstream_response_body_read(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            message,
            ErrorResponseType::UpstreamResponseBodyReadError,
        )
    }

    pub(crate) fn stream_translation(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::BAD_GATEWAY,
            message,
            ErrorResponseType::StreamTranslationError,
        )
    }

    delegate! {
        to self.payload {
            pub(crate) fn encode_sse_event(self) -> serde_json::Result<Bytes>;
            pub(crate) fn encode_sse_event_or_fallback(self) -> Bytes;
        }
    }
}

impl Error {
    pub(crate) fn response_spec(&self) -> ErrorResponseSpec {
        match self {
            Error::Request(error) => {
                let message = match error {
                    RequestError::Body(error) => error.to_string(),
                    RequestError::Invalid(message) => message.clone(),
                };
                ErrorResponseSpec::new(ErrorResponseFields::invalid_request(message))
            }
            Error::Upstream(error) => upstream_error_response_spec(error),
            Error::Config(error) => {
                ErrorResponseSpec::new(ErrorResponseFields::internal(error.to_string()))
            }
            Error::Internal(error) => {
                ErrorResponseSpec::new(ErrorResponseFields::internal(error.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ErrorResponseSpec {
    fields: ErrorResponseFields,
    headers: HeaderMap,
}

impl ErrorResponseSpec {
    fn new(fields: ErrorResponseFields) -> Self {
        Self {
            fields,
            headers: HeaderMap::new(),
        }
    }

    fn with_forwardable_headers(fields: ErrorResponseFields, source: &HeaderMap) -> Self {
        let mut headers = HeaderMap::new();
        for (name, value) in source {
            if is_forwardable_error_response_header(name) {
                headers.append(name, value.clone());
            }
        }
        Self { fields, headers }
    }

    pub(crate) fn into_response(self, format: ErrorResponseFormat) -> Response<Body> {
        let mut response = render_http_error_response(format, self.fields);
        response.headers_mut().extend(self.headers);
        response
    }
}

fn upstream_error_response_spec(error: &UpstreamError) -> ErrorResponseSpec {
    match error {
        UpstreamError::RequestSend(error) => ErrorResponseSpec::new(
            ErrorResponseFields::upstream_request(format!("upstream request failed: {error}")),
        ),
        UpstreamError::ErrorStatus { head, parsed, .. } => {
            ErrorResponseSpec::with_forwardable_headers(
                upstream_response_error_fields(head.status, parsed),
                &head.headers,
            )
        }
        UpstreamError::ResponseBodyRead { source, .. } => {
            ErrorResponseSpec::new(ErrorResponseFields::upstream_response_body_read(format!(
                "upstream response body read failed: {source}"
            )))
        }
    }
}

fn upstream_response_error_fields(
    status: StatusCode,
    error: &UpstreamResponseError,
) -> ErrorResponseFields {
    match error {
        UpstreamResponseError::Upstream {
            code,
            message,
            param,
        } => ErrorResponseFields::new_with_details(
            status,
            message.clone(),
            ErrorResponseType::UpstreamError,
            code.clone(),
            param.clone(),
        ),
        UpstreamResponseError::EmptyBody => ErrorResponseFields::new(
            status,
            "upstream error response body is empty",
            ErrorResponseType::UpstreamErrorBodyEmpty,
        ),
        UpstreamResponseError::NonJsonBody { text } => ErrorResponseFields::new(
            status,
            format!("upstream error response body is not JSON: {text}"),
            ErrorResponseType::UpstreamErrorBodyNonJson,
        ),
        UpstreamResponseError::UnknownBodyShape { text } => ErrorResponseFields::new(
            status,
            format!("upstream error response body has unknown shape: {text}"),
            ErrorResponseType::UpstreamErrorBodyUnknownShape,
        ),
    }
}

fn render_http_error_response(
    format: ErrorResponseFormat,
    fields: ErrorResponseFields,
) -> Response<Body> {
    match format {
        ErrorResponseFormat::Text => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            response_with_headers(
                fields.http_status,
                headers,
                Body::from(fields.payload.message),
            )
        }
        ErrorResponseFormat::Json => {
            let body = serde_json::to_vec(&ErrorJsonResponse {
                error: fields.payload,
            })
            .expect("serialize error response");
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response_with_headers(fields.http_status, headers, Body::from(body))
        }
    }
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
