use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::Response;
use axum::response::IntoResponse;
use axum::{Router, routing::any};
use observe::{CaptureController, InboundRequestReceived, RequestFailed};

use getset::{CopyGetters, Getters};
use std::collections::{BTreeMap, BTreeSet};
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
use observe::ObserveContext;
pub use observe::TOOL_NAME_ALIASES;
use pipeline::{InboundHttpFlow, run_provider_flow};
use protocol::ProviderProtocol;
use provider::ProviderTransport;
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
            error.into_response_with_format(format)
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
