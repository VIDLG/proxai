use axum::http::{HeaderMap, HeaderValue, Method, Uri};
use http::header::AUTHORIZATION;
use reqwest::{Client, RequestBuilder, Url};

use crate::config::{normalize_provider_name, ProviderCompatibility, ProviderConfig};
use crate::error::{InternalError, Result};
use crate::protocol::ProviderProtocol;
use crate::provider::ForwardedRequest;

#[derive(Debug, Clone)]
pub(crate) struct ProviderRuntime {
    pub(crate) name: String,
    pub(crate) protocol: ProviderProtocol,
    pub(crate) base_url: Url,
    pub(crate) api_key: String,
    pub(crate) compatibility: ProviderCompatibility,
    pub(crate) client: Client,
}

#[derive(Debug)]
pub(crate) struct PreparedProviderRequest {
    pub(crate) url: Url,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Vec<u8>,
}

impl PreparedProviderRequest {
    pub(crate) fn build(self, client: &reqwest::Client, method: Method) -> RequestBuilder {
        client
            .request(method, self.url)
            .headers(self.headers)
            .body(self.body)
    }
}

impl ProviderRuntime {
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
            client,
        })
    }

    pub(crate) fn prepare_request(
        &self,
        uri: &Uri,
        inbound_headers: &HeaderMap,
        forwarded_request: ForwardedRequest,
    ) -> Result<PreparedProviderRequest, InternalError> {
        let body_len = forwarded_request.body().len();
        let upstream_path = match uri.query() {
            Some(query) => format!("{}?{}", forwarded_request.upstream_path(), query),
            None => forwarded_request.upstream_path().to_string(),
        };
        Ok(PreparedProviderRequest {
            url: self.upstream_url_for_path(&upstream_path)?,
            headers: self.forwarded_request_headers(inbound_headers, body_len),
            body: forwarded_request.into_body(),
        })
    }

    pub(crate) fn upstream_url_for_path(&self, path_and_query: &str) -> Result<Url, InternalError> {
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

    pub(crate) fn forwarded_request_headers(
        &self,
        headers: &HeaderMap,
        body_len: usize,
    ) -> HeaderMap {
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
#[path = "runtime_tests.rs"]
mod tests;
