use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Method, Response, Uri};
use getset::{CopyGetters, Getters};
use http::header::AUTHORIZATION;
use reqwest::{Client, Url};

use crate::capture::CaptureSession;
use crate::config::{
    normalize_provider_name, ErrorResponseFormat, ProviderCompatibility, ProviderConfig,
};
use crate::error::{InternalError, Result};
use crate::protocol::ProviderProtocol;
use crate::provider::{ForwardedRequest, OutboundResponseContext};

#[derive(Debug, Clone, Getters, CopyGetters)]
pub(crate) struct ProviderTransport {
    #[getset(get = "pub(crate)")]
    name: String,
    #[getset(get_copy = "pub(crate)")]
    protocol: ProviderProtocol,
    base_url: Url,
    api_key: String,
    compatibility: ProviderCompatibility,
    read_idle_timeout: Duration,
    client: Client,
}

pub(crate) struct ProviderSendContext<'a> {
    request_id: u64,
    started: Instant,
    capture: &'a CaptureSession,
    span: &'a tracing::Span,
    sse_tool_call_timeout: Option<Duration>,
    error_response_format: ErrorResponseFormat,
}

impl<'a> ProviderSendContext<'a> {
    pub(crate) fn new(
        request_id: u64,
        started: Instant,
        capture: &'a CaptureSession,
        span: &'a tracing::Span,
        sse_tool_call_timeout: Option<Duration>,
        error_response_format: ErrorResponseFormat,
    ) -> Self {
        Self {
            request_id,
            started,
            capture,
            span,
            sse_tool_call_timeout,
            error_response_format,
        }
    }
}

pub(crate) struct ProviderSendRequest<'a> {
    method: Method,
    uri: &'a Uri,
    inbound_headers: &'a HeaderMap,
    forwarded_request: ForwardedRequest,
}

impl<'a> ProviderSendRequest<'a> {
    pub(crate) fn new(
        method: Method,
        uri: &'a Uri,
        inbound_headers: &'a HeaderMap,
        forwarded_request: ForwardedRequest,
    ) -> Self {
        Self {
            method,
            uri,
            inbound_headers,
            forwarded_request,
        }
    }

    fn method(&self) -> &Method {
        &self.method
    }

    fn upstream_path(&self) -> String {
        match self.uri.query() {
            Some(query) => format!("{}?{}", self.forwarded_request.upstream_path(), query),
            None => self.forwarded_request.upstream_path().to_string(),
        }
    }

    fn inbound_headers(&self) -> &HeaderMap {
        self.inbound_headers
    }

    fn body_len(&self) -> usize {
        self.forwarded_request.body().len()
    }

    fn capture_payload(&self) -> &serde_json::Value {
        self.forwarded_request.capture_payload()
    }

    fn into_body(self) -> Vec<u8> {
        self.forwarded_request.into_body()
    }
}

impl ProviderTransport {
    pub(crate) fn build(name: String, config: ProviderConfig) -> Result<Self, InternalError> {
        let normalized_name = normalize_provider_name(&name);
        let mut base_url = config.base_url.clone();
        base_url.set_query(None);
        base_url.set_fragment(None);
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        let mut client_builder = reqwest::Client::builder().read_timeout(config.read_idle_timeout);
        if is_loopback_url(&base_url) {
            client_builder = client_builder.no_proxy();
        }
        let client = client_builder
            .build()
            .map_err(InternalError::HttpClientBuild)?;
        Ok(Self {
            name: normalized_name,
            protocol: config.protocol,
            base_url,
            api_key: config.api_key,
            compatibility: config.compatibility,
            read_idle_timeout: config.read_idle_timeout,
            client,
        })
    }

    pub(crate) async fn send(
        &self,
        request: ProviderSendRequest<'_>,
        request_context: ProviderSendContext<'_>,
    ) -> Result<Response<Body>, crate::error::Error> {
        let upstream_url = self.upstream_url_for_path(&request.upstream_path())?;
        let forwarded_headers =
            self.forwarded_request_headers(request.inbound_headers(), request.body_len());
        let forwarded_request_capture_payload = request.capture_payload().clone();
        let method = request.method().clone();
        let forwarded_body = request.into_body();

        request_context
            .capture
            .capture_forwarded_request(
                &method,
                upstream_url.as_str(),
                &forwarded_headers,
                &forwarded_body,
                Some(&forwarded_request_capture_payload),
            )
            .await?;

        let upstream_response = self
            .client
            .request(method, upstream_url)
            .headers(forwarded_headers)
            .body(forwarded_body)
            .send()
            .await?;
        OutboundResponseContext {
            request_id: request_context.request_id,
            started: request_context.started,
            capture: request_context.capture,
            span: request_context.span,
            sse_tool_call_timeout: request_context.sse_tool_call_timeout,
            read_idle_timeout: self.read_idle_timeout,
            error_response_format: request_context.error_response_format,
            provider_protocol: self.protocol,
            provider_compatibility: self.compatibility,
        }
        .handle_response(upstream_response)
        .await
    }

    fn upstream_url_for_path(&self, path_and_query: &str) -> Result<Url, InternalError> {
        let base_path = self.base_url.path().trim_matches('/');
        let request = path_and_query.trim_start_matches('/');

        // Avoid duplicating an API root that is already present in `base_url`.
        // For example, `base_url = https://api.example/v1/` and
        // `path_and_query = /v1/responses` should join as `/v1/responses`, not
        // `/v1/v1/responses`. Non-matching gateway prefixes such as `/openai`
        // are kept.
        let relative_path = if base_path.is_empty() {
            request.to_string()
        } else {
            match request.strip_prefix(base_path) {
                Some("") => String::new(),
                Some(suffix) if suffix.starts_with('/') => {
                    suffix.trim_start_matches('/').to_string()
                }
                Some(suffix) if suffix.starts_with('?') => suffix.to_string(),
                _ => request.to_string(),
            }
        };

        Ok(self.base_url.join(&relative_path)?)
    }

    fn forwarded_request_headers(&self, headers: &HeaderMap, body_len: usize) -> HeaderMap {
        let mut forwarded = filter_forwardable_headers(headers);
        if !forwarded.contains_key(http::header::USER_AGENT) {
            forwarded.insert(
                http::header::USER_AGENT,
                HeaderValue::from_static("ProxAI/1.0"),
            );
        }
        if body_len > 0 {
            if let Ok(value) = HeaderValue::from_str(&body_len.to_string()) {
                forwarded.insert(http::header::CONTENT_LENGTH, value);
            }
        }
        match self.protocol {
            ProviderProtocol::OpenaiResponses | ProviderProtocol::OpenaiChatCompletions => {
                forwarded.remove("x-api-key");
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", self.api_key.trim()))
                {
                    forwarded.insert(AUTHORIZATION, value);
                }
            }
            ProviderProtocol::AnthropicMessages => {
                forwarded.remove(http::header::AUTHORIZATION);
                if let Ok(value) = HeaderValue::from_str(self.api_key.trim()) {
                    forwarded.insert("x-api-key", value);
                }
            }
        }
        forwarded
    }
}

fn is_loopback_url(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    }
}

pub(crate) fn filter_forwardable_headers(headers: &HeaderMap) -> HeaderMap {
    let mut forwarded = HeaderMap::new();
    for (key, value) in headers {
        if !is_hop_by_hop_header(key.as_str()) {
            forwarded.append(key, value.clone());
        }
    }
    forwarded
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "accept-encoding"
            | "connection"
            | "content-length"
            | "host"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
