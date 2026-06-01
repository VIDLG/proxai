use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, Response, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::{http, routing::any, Router};
use capture::CaptureController;

use serde_json::json;
use std::collections::BTreeSet;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{info_span, Instrument};

pub mod capture;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod formatting;
pub mod ingress;
pub mod logging;
pub mod mcp;
pub mod paths;
pub mod protocol;
pub mod provider;
pub mod routing;
pub mod sse;
pub mod translation;
pub(crate) mod upstream;

use config::{
    CaptureConfig, DefaultProviderNamesConfig, ErrorResponseFormat, ProviderConfig, RouteConfig,
};
pub use error::Error;
use error::{InternalError, RequestError, Result};
use ingress::prepare_inbound_request;
pub use logging::TOOL_NAME_ALIASES;
use logging::{ForwardedRequestEvent, RequestBodySizes};
use protocol::RequestProtocol;
use provider::{ProviderRuntime, UpstreamResponseContext};
use routing::{resolve_route, EffectiveDefaultProviderNames, EffectiveRoute};
use translation::{translate_request, translate_response};

#[derive(Clone)]
pub struct AppState {
    default_provider_names: EffectiveDefaultProviderNames,
    providers: std::collections::BTreeMap<String, ProviderRuntime>,
    routes: Vec<EffectiveRoute>,
    error_response_format: ErrorResponseFormat,
    capture: CaptureController,
    sse_tool_call_timeout: Option<Duration>,
}

impl AppState {
    pub fn new(
        default_provider_names: DefaultProviderNamesConfig,
        providers: std::collections::BTreeMap<String, ProviderConfig>,
        routes: Vec<RouteConfig>,
    ) -> Result<Self> {
        let provider_runtimes = providers
            .into_iter()
            .map(|(name, config)| {
                let runtime = ProviderRuntime::build(name, config)?;
                Ok((runtime.name.clone(), runtime))
            })
            .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
        let provider_protocols = provider_runtimes
            .values()
            .map(|runtime| (runtime.name.clone(), runtime.protocol))
            .collect();
        let provider_names = provider_runtimes.keys().cloned().collect::<BTreeSet<_>>();
        let effective_default_provider_names =
            EffectiveDefaultProviderNames::build(default_provider_names, &provider_names)?;
        let effective_routes = EffectiveRoute::build(&provider_protocols, routes)?;

        Ok(Self {
            default_provider_names: effective_default_provider_names,
            providers: provider_runtimes,
            routes: effective_routes,
            error_response_format: ErrorResponseFormat::Text,
            capture: CaptureController::new(None, CaptureConfig::default()),
            sse_tool_call_timeout: Some(Duration::from_secs(120)),
        })
    }

    pub fn with_error_response_format(mut self, format: ErrorResponseFormat) -> Self {
        self.error_response_format = format;
        self
    }

    pub fn with_capture_dir(mut self, capture_dir: Option<std::path::PathBuf>) -> Self {
        self.capture.set_dir(capture_dir);
        self
    }

    pub fn with_capture_config(mut self, defaults: CaptureConfig) -> Self {
        self.capture.set_default_config(defaults);
        self
    }

    pub fn with_sse_tool_call_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.sse_tool_call_timeout = timeout;
        self
    }

    pub fn capture_controller(&self) -> CaptureController {
        self.capture.clone()
    }

    pub async fn serve(
        self,
        listener: tokio::net::TcpListener,
        shutdown: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<()> {
        let app = Router::new()
            .route("/v1/responses", any(proxy))
            .route("/responses", any(proxy))
            .route("/v1/chat/completions", any(proxy))
            .route("/chat/completions", any(proxy))
            .route("/v1/messages", any(proxy))
            .route("/messages", any(proxy))
            .with_state(self);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(Into::into)
    }
}

async fn proxy(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    request: Request<Body>,
) -> impl IntoResponse {
    let request_id = request_id();
    let span = info_span!("request", request_id);
    let inbound_request_context = ProxyRequestContext {
        request_id,
        started: Instant::now(),
        span: span.clone(),
    };
    let content_length = headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    span.in_scope(|| {
        tracing::info!(
            request_id,
            method = %method,
            path = uri.path(),
            content_length,
            "recv"
        );
    });

    let format = state.error_response_format;
    let response = proxy_inner(
        state,
        method,
        uri,
        headers,
        request,
        inbound_request_context,
    )
    .instrument(span)
    .await
    .unwrap_or_else(|error| error_response(error, format));
    tracing::info!(request_id, status = response.status().as_u16(), "sent");
    response
}

struct ProxyRequestContext {
    request_id: u64,
    started: Instant,
    span: tracing::Span,
}

fn request_id() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn request_protocol_for_path(path: &str) -> Result<RequestProtocol, RequestError> {
    match path {
        "/v1/responses" | "/responses" => Ok(RequestProtocol::OpenaiResponses),
        "/v1/chat/completions" | "/chat/completions" => Ok(RequestProtocol::OpenaiChatCompletions),
        "/v1/messages" | "/messages" => Ok(RequestProtocol::AnthropicMessages),
        _ => Err(RequestError::Invalid(format!(
            "unsupported request path `{path}`"
        ))),
    }
}

fn error_response(error: error::Error, format: ErrorResponseFormat) -> Response<Body> {
    tracing::warn!(error = %error, "request failed");

    match &error {
        error::Error::Request(_) => return error.into_response(),
        error::Error::Config(_) | error::Error::Internal(_) => return error.into_response(),
        error::Error::Upstream(_) => {}
    }

    let status = StatusCode::BAD_GATEWAY;
    match format {
        ErrorResponseFormat::Text => {
            let mut response = Response::new(Body::from(error.to_string()));
            *response.status_mut() = status;
            response.headers_mut().insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            response
        }
        ErrorResponseFormat::Json => {
            let body = serde_json::to_vec(&json!({
                "error": {
                    "message": error.to_string(),
                    "type": status.canonical_reason().unwrap_or("error"),
                    "code": status.as_u16(),
                }
            }))
            .expect("serialize error response");
            let mut response = Response::new(Body::from(body));
            *response.status_mut() = status;
            response.headers_mut().insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        }
    }
}

async fn proxy_inner(
    state: AppState,
    method: Method,
    uri: Uri,
    inbound_request_headers: HeaderMap,
    inbound_request: Request<Body>,
    inbound_request_context: ProxyRequestContext,
) -> Result<Response<Body>> {
    let request_started = inbound_request_context.started;
    let request_id = inbound_request_context.request_id;
    let request_span = inbound_request_context.span;
    let inbound_request_body_bytes = to_bytes(inbound_request.into_body(), usize::MAX)
        .await
        .map_err(RequestError::Body)?;
    let request_protocol = request_protocol_for_path(uri.path())?;
    let inbound_request = prepare_inbound_request(request_protocol, &inbound_request_body_bytes)?;

    let capture = state.capture.session(request_id);

    capture
        .capture_inbound_request(
            &method,
            &uri,
            &inbound_request_headers,
            &inbound_request_body_bytes,
        )
        .await?;

    let resolved_route = resolve_route(
        &state.default_provider_names,
        &state.routes,
        inbound_request.protocol(),
        inbound_request.model(),
    )?;
    let provider = state
        .providers
        .get(&resolved_route.provider_name)
        .ok_or_else(|| {
            InternalError::InvalidProviderResolution(resolved_route.provider_name.clone())
        })?;
    let forwarded_request = translate_request(
        &inbound_request,
        provider.protocol,
        &resolved_route.upstream_model,
    )?;
    let forwarded_request_view = forwarded_request.view();
    let forwarded_request_capture_payload = forwarded_request.capture_payload().clone();
    let forwarded_request_body_len = forwarded_request.body().len();

    request_span.in_scope(|| {
        ForwardedRequestEvent {
            request_id,
            method: method.clone(),
            uri: uri.clone(),
            request_sizes: RequestBodySizes {
                inbound: inbound_request_body_bytes.len() as u64,
                forwarded: forwarded_request_body_len as u64,
            },
            forwarded_request: forwarded_request_view,
            capture: capture.forwarded_request_enabled(),
        }
        .emit()
    });

    let provider_request =
        provider.prepare_request(&uri, &inbound_request_headers, forwarded_request)?;

    capture
        .capture_forwarded_request(
            &method,
            provider_request.url.as_str(),
            &provider_request.headers,
            &provider_request.body,
            Some(&forwarded_request_capture_payload),
        )
        .await?;

    let upstream_response = provider_request
        .build(&provider.client, method.clone())
        .send()
        .await?;
    let response = UpstreamResponseContext {
        request_id,
        started: request_started,
        capture: &capture,
        span: &request_span,
        sse_tool_call_timeout: state.sse_tool_call_timeout,
        error_response_format: state.error_response_format,
        provider_compatibility: provider.compatibility,
    }
    .handle_response(provider.protocol, upstream_response)
    .await?;
    Ok(translate_response(inbound_request.protocol(), provider.protocol, response).await?)
}
