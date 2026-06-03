use std::time::Duration;

use axum::http::{HeaderMap, HeaderValue, Method};
use getset::{CopyGetters, Getters};
use headers::{ContentLength, HeaderMapExt};
use reqwest::{Client, Url};

use crate::capture::CaptureSession;
use crate::config::{normalize_provider_name, ProviderCompatibility, ProviderConfig};
use crate::error::{Error as ProxyError, InternalError, Result, UpstreamError};
use crate::http_utils::filter_forwardable_headers;
use crate::protocol::ProviderProtocol;
use crate::provider::{apply_request_auth_headers, ProviderRequest};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProviderTransportError {
    #[error(transparent)]
    Internal(#[from] InternalError),
    #[error(transparent)]
    Upstream(#[from] UpstreamError),
}

impl From<ProviderTransportError> for ProxyError {
    fn from(error: ProviderTransportError) -> Self {
        match error {
            ProviderTransportError::Internal(error) => error.into(),
            ProviderTransportError::Upstream(error) => error.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, CopyGetters)]
pub(crate) struct ProviderStreamingResponsePolicy {
    #[getset(get_copy = "pub(crate)")]
    read_idle_timeout: Duration,
    #[getset(get_copy = "pub(crate)")]
    sse_tool_call_timeout: Option<Duration>,
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub(crate) struct ProviderTransport {
    #[getset(get = "pub(crate)")]
    name: String,
    #[getset(get_copy = "pub(crate)")]
    protocol: ProviderProtocol,
    #[getset(get_copy = "pub(crate)")]
    compatibility: ProviderCompatibility,
    #[getset(get_copy = "pub(crate)")]
    read_idle_timeout: Duration,
    #[getset(get_copy = "pub(crate)")]
    sse_tool_call_timeout: Option<Duration>,
    base_url: Url,
    api_key: String,
    client: Client,
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
            compatibility: config.compatibility,
            read_idle_timeout: config.read_idle_timeout,
            sse_tool_call_timeout: Some(Duration::from_secs(120)),
            base_url,
            api_key: config.api_key,
            client,
        })
    }

    pub(crate) fn streaming_response_policy(&self) -> ProviderStreamingResponsePolicy {
        ProviderStreamingResponsePolicy {
            read_idle_timeout: self.read_idle_timeout,
            sse_tool_call_timeout: self.sse_tool_call_timeout,
        }
    }

    pub(crate) fn set_sse_tool_call_timeout(&mut self, timeout: Option<Duration>) {
        self.sse_tool_call_timeout = timeout;
    }

    pub(crate) async fn send(
        &self,
        method: Method,
        inbound_query: Option<String>,
        inbound_headers: HeaderMap,
        provider_request: ProviderRequest,
        capture: &CaptureSession,
    ) -> std::result::Result<reqwest::Response, ProviderTransportError> {
        let upstream_path = match inbound_query {
            Some(query) => format!("{}?{}", provider_request.upstream_path(), query),
            None => provider_request.upstream_path().to_string(),
        };
        let url = upstream_url_for_path(&self.base_url, &upstream_path)?;
        let headers = provider_request_headers(
            &inbound_headers,
            provider_request.body().len(),
            self.protocol,
            &self.api_key,
        );
        let capture_payload = provider_request.capture_payload().clone();
        let body = provider_request.into_body();

        capture
            .capture_provider_request(
                &method,
                url.as_str(),
                &headers,
                &body,
                Some(&capture_payload),
            )
            .await;

        self.client
            .request(method, url)
            .headers(headers)
            .body(body)
            .send()
            .await
            .map_err(UpstreamError::RequestSend)
            .map_err(Into::into)
    }
}

fn provider_request_headers(
    headers: &HeaderMap,
    body_len: usize,
    protocol: ProviderProtocol,
    api_key: &str,
) -> HeaderMap {
    let mut provider_headers = filter_forwardable_headers(headers);
    if !provider_headers.contains_key(http::header::USER_AGENT) {
        provider_headers.insert(
            http::header::USER_AGENT,
            HeaderValue::from_static("ProxAI/1.0"),
        );
    }
    if body_len > 0 {
        provider_headers.typed_insert(ContentLength(body_len as u64));
    }
    apply_request_auth_headers(protocol, &mut provider_headers, api_key);
    provider_headers
}

fn upstream_url_for_path(base_url: &Url, path_and_query: &str) -> Result<Url, InternalError> {
    let base_path = base_url.path().trim_matches('/');
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
            Some(suffix) if suffix.starts_with('/') => suffix.trim_start_matches('/').to_string(),
            Some(suffix) if suffix.starts_with('?') => suffix.to_string(),
            _ => request.to_string(),
        }
    };

    Ok(base_url.join(&relative_path)?)
}

fn is_loopback_url(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain("localhost")) => true,
        Some(url::Host::Ipv4(address)) => address.is_loopback(),
        Some(url::Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
