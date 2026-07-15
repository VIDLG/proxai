use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::Response;
use axum::response::IntoResponse;
use axum::{Router, routing::any};
use observe::{CaptureController, InboundRequestReceived, RequestFailed};

use getset::{CopyGetters, Getters};
use std::collections::BTreeMap;
use std::time::Duration;
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;

pub mod config;
pub mod error;
pub mod formatting;
pub mod http_support;
pub mod ingress;
pub mod mcp;
pub mod observe;
pub mod paths;
pub(crate) mod pipeline;
pub use proxai_core::protocol;
pub mod provider;
pub mod request;
pub use proxai_core::routing;
pub mod sse;
mod sse_translation;
pub use proxai_core::translation;
pub(crate) mod upstream;

use config::{CaptureConfig, ErrorResponseFormat, ProviderConfig, RoutingConfig};
pub use error::Error;
use error::{InternalError, RequestError, Result};
use observe::ObserveContext;
pub use observe::TOOL_NAME_ALIASES;
use pipeline::{InboundHttpFlow, run_provider_flow};
use provider::ProviderTransport;
use routing::RoutingTable;

#[derive(Clone, Getters, CopyGetters)]
pub struct AppState {
    routing: RoutingTable,
    providers: BTreeMap<String, ProviderTransport>,
    #[getset(get_copy = "pub(crate)")]
    error_response_format: ErrorResponseFormat,
    #[getset(get = "pub(crate)")]
    capture: CaptureController,
    max_request_body_bytes: usize,
    max_concurrent_requests: usize,
}

impl AppState {
    pub fn new(
        routing: RoutingConfig,
        providers: BTreeMap<String, ProviderConfig>,
    ) -> Result<Self> {
        let routing =
            RoutingTable::build(routing, providers.keys()).map_err(InternalError::from)?;
        let provider_transports = providers
            .into_iter()
            .map(|(name, config)| {
                let transport = ProviderTransport::build(name, config)?;
                Ok((transport.name().to_string(), transport))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        Ok(Self {
            routing,
            providers: provider_transports,
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
            .map_err(InternalError::Io)
            .map_err(Into::into)
    }
}

async fn proxy(State(state): State<AppState>, request: Request<Body>) -> impl IntoResponse {
    let obs = ObserveContext::start(state.capture().clone());
    obs.observe_inbound_request_received(InboundRequestReceived {
        method: request.method(),
        uri: request.uri(),
        headers: request.headers(),
    });

    let format = state.error_response_format();

    match obs
        .instrument(proxy_inner(state, request, obs.clone()))
        .await
    {
        Ok(response) => response,
        Err(error) => {
            obs.observe_request_failed(RequestFailed { error: &error });
            error.response_spec().into_response(format)
        }
    }
}

async fn proxy_inner(
    state: AppState,
    inbound_request: Request<Body>,
    obs: ObserveContext,
) -> Result<Response<Body>> {
    let (inbound_request_parts, inbound_body) = inbound_request.into_parts();
    let body_bytes = to_bytes(inbound_body, usize::MAX)
        .await
        .map_err(RequestError::Body)?;
    let inbound_http = InboundHttpFlow::new(
        inbound_request_parts,
        body_bytes,
        obs,
        state.error_response_format(),
    );
    let prepared_provider = inbound_http
        .prepare_inbound()?
        .route_to_provider(&state.routing, &state.providers)?
        .prepare_provider_request()?;

    run_provider_flow(prepared_provider).await
}
