use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::handler::Handler;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, Uri};
use axum::response::{IntoResponse, Json};
use futures_util::StreamExt;
use http::header::AUTHORIZATION;
use reqwest::{Client, Url};
use serde_json::Value;
use std::future::Future;
use thiserror::Error;
use tokio::net::TcpListener;
use tracing::{error, info};

mod normalize;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("invalid HTTP method: {0}")]
    InvalidMethod(#[from] http::method::InvalidMethod),

    #[error("request body error: {0}")]
    Body(#[from] axum::Error),

    #[error("serialize request body: {0}")]
    Json(#[from] serde_json::Error),

    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response<Body> {
        error!(error = %self, "shim exception");
        (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": {
                    "message": self.to_string(),
                    "type": "shim_error"
                }
            })),
        )
            .into_response()
    }
}

#[derive(Clone)]
pub struct AppState {
    upstream: Url,
    api_key: Option<String>,
    override_authorization: bool,
    client: Client,
}

impl AppState {
    pub fn new(
        upstream: impl Into<String>,
        api_key: Option<String>,
        override_authorization: bool,
        client: Client,
    ) -> Result<Self> {
        let mut upstream = Url::parse(&upstream.into())?;
        upstream.set_query(None);
        upstream.set_fragment(None);
        if !upstream.path().ends_with('/') {
            upstream.set_path(&format!("{}/", upstream.path()));
        }
        Ok(Self {
            upstream,
            api_key,
            override_authorization,
            client,
        })
    }

    pub async fn serve<F>(self, listener: TcpListener, shutdown_signal: F) -> std::io::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        axum::serve(listener, proxy.with_state(self))
            .with_graceful_shutdown(shutdown_signal)
            .await
    }

    fn upstream_url(&self, uri: &Uri) -> Result<Url> {
        Ok(self.upstream.join(
            uri.path_and_query()
                .map(http::uri::PathAndQuery::as_str)
                .unwrap_or("/")
                .trim_start_matches('/'),
        )?)
    }

    fn forwarded_headers(&self, headers: &HeaderMap, body_len: usize) -> HeaderMap {
        let mut forwarded = HeaderMap::new();
        for (key, value) in headers {
            if !is_hop_by_hop_header(key.as_str()) && key != http::header::ACCEPT_ENCODING {
                forwarded.append(key, value.clone());
            }
        }
        if !forwarded.contains_key(http::header::USER_AGENT) {
            forwarded.insert(
                http::header::USER_AGENT,
                HeaderValue::from_static("Zed-OpenAI-Shim/1.0"),
            );
        }
        if body_len > 0 {
            if let Ok(value) = HeaderValue::from_str(&body_len.to_string()) {
                forwarded.insert(http::header::CONTENT_LENGTH, value);
            }
        }
        if let Some(api_key) = self
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if self.override_authorization || !forwarded.contains_key(AUTHORIZATION) {
                if let Ok(value) = HeaderValue::from_str(&format!("Bearer {api_key}")) {
                    forwarded.insert(AUTHORIZATION, value);
                }
            }
        }
        forwarded
    }
}

async fn proxy(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    request: Request<Body>,
) -> Result<Response<Body>> {
    let body_bytes = to_bytes(request.into_body(), usize::MAX).await?;
    let (body, summary) = if let Ok(payload) = serde_json::from_slice::<Value>(&body_bytes) {
        let payload = normalize::normalize_payload(payload);
        (serde_json::to_vec(&payload)?, request_summary(&payload))
    } else {
        (body_bytes.to_vec(), None)
    };

    info!(
        method = %method,
        path = %uri,
        bytes = body.len(),
        summary = summary.as_deref().unwrap_or(""),
        "forward"
    );

    let mut upstream_request = state
        .client
        .request(
            reqwest::Method::from_bytes(method.as_str().as_bytes())?,
            state.upstream_url(&uri)?,
        )
        .headers(state.forwarded_headers(&headers, body.len()));

    if !body.is_empty()
        || matches!(
            method,
            Method::POST | Method::PUT | Method::PATCH | Method::DELETE
        )
    {
        upstream_request = upstream_request.body(body);
    }

    let upstream = upstream_request.send().await?;
    info!(path = %uri, status = upstream.status().as_u16(), "upstream");

    let status = upstream.status();
    let mut headers = HeaderMap::new();
    for (key, value) in upstream.headers() {
        if !is_hop_by_hop_header(key.as_str()) {
            headers.append(key, value.clone());
        }
    }

    let stream = upstream
        .bytes_stream()
        .map(|chunk| chunk.map_err(std::io::Error::other));
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    for (key, value) in headers {
        if let Some(key) = key {
            response.headers_mut().append(key, value);
        }
    }
    Ok(response)
}

fn request_summary(payload: &Value) -> Option<String> {
    let object = payload.as_object()?;
    let fields = [
        "model",
        "stream",
        "max_output_tokens",
        "max_completion_tokens",
    ]
    .into_iter()
    .filter_map(|key| object.get(key).map(|value| format!("{key}={value}")))
    .collect::<Vec<_>>();
    (!fields.is_empty()).then(|| fields.join(" "))
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
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
