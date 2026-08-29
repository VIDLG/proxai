use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::Response;
use axum::response::IntoResponse;
use axum::{Router, routing::any};
use observe::{CaptureController, InboundRequestReceived, RequestFailed};

use getset::{CopyGetters, Getters};
use proxai_core::pipeline::Pipeline as CorePipeline;
use proxai_core::provider::ProviderBehavior;
use proxai_core::routing::normalize_provider_name;
use std::collections::BTreeMap;
use std::time::Duration;
use tower::ServiceBuilder;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::limit::RequestBodyLimitLayer;

pub mod config;
pub mod error;
pub mod formatting;
pub mod http_support;
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

#[doc(hidden)]
pub fn ensure_rustls_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

use config::{CaptureConfig, ErrorResponseFormat, ProviderConfig, ProxyConfig, RoutingConfig};
pub use error::Error;
use error::{InternalError, RequestError, Result};
use observe::ObserveContext;
pub use observe::TOOL_NAME_ALIASES;
use pipeline::{InboundHttpFlow, run_provider_flow};
use provider::ProviderTransport;

#[derive(Clone, Getters, CopyGetters)]
pub struct AppState {
    core_pipeline: CorePipeline,
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
        Self::new_with_proxies(routing, providers, BTreeMap::new())
    }

    pub fn new_with_proxies(
        routing: RoutingConfig,
        providers: BTreeMap<String, ProviderConfig>,
        proxies: BTreeMap<String, ProxyConfig>,
    ) -> Result<Self> {
        let provider_behaviors = providers
            .iter()
            .map(|(name, config)| {
                (
                    name.clone(),
                    ProviderBehavior::new(config.protocol, config.compatibility),
                )
            })
            .collect();
        let core_pipeline =
            CorePipeline::build(routing, provider_behaviors).map_err(InternalError::from)?;
        let proxies = proxies
            .into_iter()
            .map(|(name, config)| (normalize_provider_name(&name), config))
            .collect::<BTreeMap<_, _>>();
        let provider_transports = providers
            .into_iter()
            .map(|(name, config)| {
                let proxy = match config.proxy.as_deref() {
                    Some(proxy_name) => Some(
                        proxies
                            .get(&normalize_provider_name(proxy_name))
                            .ok_or_else(|| InternalError::MissingProxyConfig {
                                provider: name.clone(),
                                proxy: proxy_name.to_string(),
                            })?,
                    ),
                    None => None,
                };
                let transport = ProviderTransport::build(name, config, proxy)?;
                Ok((transport.name().to_string(), transport))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        Ok(Self {
            core_pipeline,
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
        .parse_inbound()?
        .prepare_provider_request(&state.core_pipeline, &state.providers)?;

    run_provider_flow(prepared_provider).await
}
