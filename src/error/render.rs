use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Response, StatusCode, header};

use crate::config::ErrorResponseFormat;
use crate::http_support::{is_forwardable_error_response_header, response_with_headers};

use super::{Error, RequestError, UpstreamError, UpstreamResponseError};

impl Error {
    pub(crate) fn into_response_with_format(self, format: ErrorResponseFormat) -> Response<Body> {
        let (status, message, error_type, response_format) = match self {
            Error::Request(error) => {
                let message = match error {
                    RequestError::Body(error) => error.to_string(),
                    RequestError::Invalid(message) => message,
                };
                (
                    StatusCode::BAD_REQUEST,
                    message,
                    "invalid_request_error",
                    ErrorResponseFormat::Json,
                )
            }
            Error::Upstream(error) => return upstream_error_response(format, *error),
            Error::Config(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
                "internal_error",
                ErrorResponseFormat::Json,
            ),
            Error::Internal(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error.to_string(),
                "internal_error",
                ErrorResponseFormat::Json,
            ),
        };

        generic_error_response(response_format, status, message, error_type)
    }
}

fn upstream_error_response(format: ErrorResponseFormat, error: UpstreamError) -> Response<Body> {
    match error {
        UpstreamError::RequestSend(error) => generic_error_response(
            format,
            StatusCode::BAD_GATEWAY,
            format!("upstream request failed: {error}"),
            "upstream_request_error",
        ),
        UpstreamError::ErrorStatus { head, parsed, .. } => {
            let mut response = upstream_status_error_response(format, head.status, &parsed);
            for (name, value) in &head.headers {
                if is_forwardable_error_response_header(name) {
                    response.headers_mut().append(name, value.clone());
                }
            }
            response
        }
        UpstreamError::ResponseBodyRead { source, .. } => generic_error_response(
            format,
            StatusCode::BAD_GATEWAY,
            format!("upstream response body read failed: {source}"),
            "upstream_response_body_read_error",
        ),
    }
}

fn upstream_status_error_response(
    format: ErrorResponseFormat,
    status: StatusCode,
    error: &UpstreamResponseError,
) -> Response<Body> {
    match format {
        ErrorResponseFormat::Text => generic_error_response(
            format,
            status,
            format!("upstream {}: {}", status.as_u16(), error.response_message()),
            error.response_type(),
        ),
        ErrorResponseFormat::Json => generic_error_response(
            format,
            status,
            error.response_message(),
            error.response_type(),
        ),
    }
}

fn generic_error_response(
    format: ErrorResponseFormat,
    status: StatusCode,
    message: String,
    error_type: &'static str,
) -> Response<Body> {
    match format {
        ErrorResponseFormat::Text => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            response_with_headers(status, headers, Body::from(message))
        }
        ErrorResponseFormat::Json => {
            let body = serde_json::to_vec(&serde_json::json!({
                "error": {
                    "message": message,
                    "type": error_type,
                    "status": status.as_u16(),
                }
            }))
            .expect("serialize error response");
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response_with_headers(status, headers, Body::from(body))
        }
    }
}
