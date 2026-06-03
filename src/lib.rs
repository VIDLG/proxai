use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::Response;
use axum::response::IntoResponse;
use axum::{routing::any, Router};
use capture::{CaptureController, CaptureSession};

use getset::{CopyGetters, Getters};
use headers::{ContentLength, HeaderMapExt};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower::limit::ConcurrencyLimitLayer;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{field::Empty, info_span, Instrument};

pub mod capture;
pub mod config;
pub mod diagnostics;
pub mod error;
pub mod formatting;
pub mod http_model;
pub(crate) mod http_utils;
pub mod ingress;
pub mod logging;
pub mod mcp;
pub mod paths;
pub(crate) mod pipeline;
pub mod protocol;
pub mod provider;
pub mod request;
pub mod routing;
pub mod sse;
pub mod translation;
pub(crate) mod upstream;

use config::{
    CaptureConfig, DefaultProviderNamesConfig, ErrorResponseFormat, ProviderConfig, RouteConfig,
};
pub use error::Error;
use error::{InternalError, RequestError, Result};
pub use logging::TOOL_NAME_ALIASES;
use pipeline::{run_provider_flow, InboundHttpFlow};
use protocol::ProviderProtocol;
use provider::ProviderTransport;
use request::RequestId;
use routing::{EffectiveDefaultProviderNames, EffectiveRoute};

#[derive(Clone, Getters, CopyGetters)]
pub struct AppState {
    provider_protocols: BTreeMap<String, ProviderProtocol>,
    default_provider_names: EffectiveDefaultProviderNames,
    providers: BTreeMap<String, ProviderTransport>,
    routes: Vec<EffectiveRoute>,
    #[getset(get_copy = "pub(crate)")]
    error_response_format: ErrorResponseFormat,
    #[getset(get = "pub(crate)")]
    capture: CaptureController,
    max_request_body_bytes: usize,
    max_concurrent_requests: usize,
}

impl AppState {
    pub fn new(
        default_provider_names: DefaultProviderNamesConfig,
        providers: BTreeMap<String, ProviderConfig>,
        routes: Vec<RouteConfig>,
    ) -> Result<Self> {
        let provider_transports = providers
            .into_iter()
            .map(|(name, config)| {
                let transport = ProviderTransport::build(name, config)?;
                Ok((transport.name().to_string(), transport))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let provider_protocols = provider_transports
            .values()
            .map(|transport| (transport.name().to_string(), transport.protocol()))
            .collect();
        let provider_names = provider_transports.keys().cloned().collect::<BTreeSet<_>>();
        let effective_default_provider_names =
            EffectiveDefaultProviderNames::build(default_provider_names, &provider_names)?;
        let effective_routes = EffectiveRoute::build(&provider_protocols, routes)?;

        Ok(Self {
            provider_protocols,
            default_provider_names: effective_default_provider_names,
            providers: provider_transports,
            routes: effective_routes,
            error_response_format: ErrorResponseFormat::Text,
            capture: CaptureController::new(None, CaptureConfig::default()),
            max_request_body_bytes: 50 * 1024 * 1024,
            max_concurrent_requests: 64,
        })
    }

    pub fn with_server_limits(
        mut self,
        max_request_body_bytes: usize,
        max_concurrent_requests: usize,
    ) -> Self {
        self.max_request_body_bytes = max_request_body_bytes;
        self.max_concurrent_requests = max_concurrent_requests;
        self
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
        // Unlike provider read_idle_timeout, this semantic stream timeout is not baked into
        // the reqwest client and can be applied to built provider transports.
        for provider in self.providers.values_mut() {
            provider.set_sse_tool_call_timeout(timeout);
        }
        self
    }

    pub fn capture_controller(&self) -> CaptureController {
        self.capture.clone()
    }

    pub(crate) fn provider(&self, name: &str) -> Result<&ProviderTransport> {
        self.providers
            .get(name)
            .ok_or_else(|| InternalError::InvalidProviderResolution(name.to_string()).into())
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
            .layer(
                ServiceBuilder::new()
                    .layer(ConcurrencyLimitLayer::new(self.max_concurrent_requests))
                    .layer(RequestBodyLimitLayer::new(self.max_request_body_bytes)),
            )
            .with_state(self);

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(Into::into)
    }
}

async fn proxy(State(state): State<AppState>, request: Request<Body>) -> impl IntoResponse {
    let request_id = generate_request_id();
    let raw_request_id: u64 = request_id.into();
    let span = info_span!(
        "request",
        request_id = raw_request_id,
        request_reasoning_effort = Empty
    );
    let started = Instant::now();
    let capture = state.capture().session(request_id);
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let content_length = request
        .headers()
        .typed_get::<ContentLength>()
        .map(|value| value.0);

    span.in_scope(|| {
        tracing::debug!(
            event = "recv",
            request_id = raw_request_id,
            method = %method,
            path,
            content_length,
        );
    });

    let format = state.error_response_format();
    let response = proxy_inner(state, request, request_id, started, span.clone(), capture)
        .instrument(span)
        .await
        .unwrap_or_else(|error| error.into_response_with_format(format));
    response
}

async fn proxy_inner(
    state: AppState,
    inbound_request: Request<Body>,
    request_id: RequestId,
    started: Instant,
    span: tracing::Span,
    capture: CaptureSession,
) -> Result<Response<Body>> {
    let (inbound_request_parts, inbound_body) = inbound_request.into_parts();
    let body_bytes = to_bytes(inbound_body, usize::MAX)
        .await
        .map_err(RequestError::Body)?;
    let inbound_http = InboundHttpFlow::new(
        inbound_request_parts,
        body_bytes,
        request_id,
        started,
        span,
        capture,
        state.error_response_format(),
    );
    let prepared_provider = inbound_http
        .prepare_inbound()
        .await?
        .route_to_provider(
            &state.default_provider_names,
            &state.routes,
            &state.provider_protocols,
        )?
        .translate_to_provider()?;

    let transport = state.provider(prepared_provider.provider_name())?;

    run_provider_flow(prepared_provider, transport).await
}

fn generate_request_id() -> RequestId {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
        .into()
}
