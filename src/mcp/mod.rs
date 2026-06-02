use axum::{routing::get, Router};
use rmcp::transport::{
    streamable_http_server::{session::local::LocalSessionManager, tower::StreamableHttpService},
    StreamableHttpServerConfig,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;
use std::net::SocketAddr;

use crate::capture::{CaptureController, CaptureQuery, CaptureShowTarget};
use crate::error::Result;

pub const MCP_PATH: &str = "/mcp";

#[derive(Clone)]
pub struct ProxaiMcpServer {
    capture: CaptureController,
}

impl ProxaiMcpServer {
    pub fn new(capture: CaptureController) -> Self {
        Self { capture }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaptureTarget {
    InboundRequest,
    ForwardedRequest,
    UpstreamResponse,
    OutboundResponse,
    All,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CaptureToggleParams {
    #[schemars(description = "Which capture phase to change. Defaults to all.")]
    pub target: Option<CaptureTarget>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CaptureShowParams {
    #[schemars(
        description = "Which capture artifact group to show. Omit for inbound_request, forwarded_request, upstream_response, and outbound_response."
    )]
    pub target: Option<CaptureShowSingleTarget>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaptureShowSingleTarget {
    InboundRequest,
    ForwardedRequest,
    UpstreamResponse,
    OutboundResponse,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CaptureListParams {
    #[schemars(
        description = "Maximum number of recent capture records to return. Defaults to 10."
    )]
    pub limit: Option<usize>,
}

#[tool_router]
impl ProxaiMcpServer {
    #[tool(
        description = "Show current proxai capture defaults, runtime overrides, and effective state."
    )]
    fn capture_status(&self) -> String {
        let status = self.capture.status();
        let mut lines = vec![
            format!(
                "defaults.inbound_request_enabled: {}",
                status.defaults.inbound_request_enabled
            ),
            format!(
                "defaults.forwarded_request_enabled: {}",
                status.defaults.forwarded_request_enabled
            ),
            format!(
                "defaults.upstream_response_enabled: {}",
                status.defaults.upstream_response_enabled
            ),
            format!(
                "defaults.outbound_response_enabled: {}",
                status.defaults.outbound_response_enabled
            ),
            format!(
                "overrides.inbound_request_enabled: {}",
                status
                    .overrides
                    .inbound_request_enabled
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "overrides.forwarded_request_enabled: {}",
                status
                    .overrides
                    .forwarded_request_enabled
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "overrides.upstream_response_enabled: {}",
                status
                    .overrides
                    .upstream_response_enabled
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "overrides.outbound_response_enabled: {}",
                status
                    .overrides
                    .outbound_response_enabled
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            format!(
                "effective.inbound_request_enabled: {}",
                status.effective.inbound_request_enabled
            ),
            format!(
                "effective.forwarded_request_enabled: {}",
                status.effective.forwarded_request_enabled
            ),
            format!(
                "effective.upstream_response_enabled: {}",
                status.effective.upstream_response_enabled
            ),
            format!(
                "effective.outbound_response_enabled: {}",
                status.effective.outbound_response_enabled
            ),
        ];

        if let Some(dir) = status.captures_dir.as_ref() {
            lines.push(format!("captures_dir: {}", dir.display()));
        }

        lines.join("\n")
    }

    #[tool(description = "Enable proxai capture overrides for one phase or all phases.")]
    fn capture_enable(&self, Parameters(params): Parameters<CaptureToggleParams>) -> String {
        match params.target.unwrap_or(CaptureTarget::All) {
            CaptureTarget::InboundRequest => self
                .capture
                .set_inbound_request_enabled_override(Some(true)),
            CaptureTarget::ForwardedRequest => self
                .capture
                .set_forwarded_request_enabled_override(Some(true)),
            CaptureTarget::UpstreamResponse => self
                .capture
                .set_upstream_response_enabled_override(Some(true)),
            CaptureTarget::OutboundResponse => self
                .capture
                .set_outbound_response_enabled_override(Some(true)),
            CaptureTarget::All => self
                .capture
                .set_overrides(crate::capture::CaptureOverrides {
                    inbound_request_enabled: Some(true),
                    forwarded_request_enabled: Some(true),
                    upstream_response_enabled: Some(true),
                    outbound_response_enabled: Some(true),
                }),
        }
        self.capture_status()
    }

    #[tool(description = "Disable proxai capture overrides for one phase or all phases.")]
    fn capture_disable(&self, Parameters(params): Parameters<CaptureToggleParams>) -> String {
        match params.target.unwrap_or(CaptureTarget::All) {
            CaptureTarget::InboundRequest => self
                .capture
                .set_inbound_request_enabled_override(Some(false)),
            CaptureTarget::ForwardedRequest => self
                .capture
                .set_forwarded_request_enabled_override(Some(false)),
            CaptureTarget::UpstreamResponse => self
                .capture
                .set_upstream_response_enabled_override(Some(false)),
            CaptureTarget::OutboundResponse => self
                .capture
                .set_outbound_response_enabled_override(Some(false)),
            CaptureTarget::All => self
                .capture
                .set_overrides(crate::capture::CaptureOverrides {
                    inbound_request_enabled: Some(false),
                    forwarded_request_enabled: Some(false),
                    upstream_response_enabled: Some(false),
                    outbound_response_enabled: Some(false),
                }),
        }
        self.capture_status()
    }

    #[tool(description = "Show the latest proxai capture artifact paths.")]
    fn capture_show_latest(&self, Parameters(params): Parameters<CaptureShowParams>) -> String {
        let query = CaptureQuery::Show(match params.target {
            Some(CaptureShowSingleTarget::InboundRequest) => {
                Some(CaptureShowTarget::InboundRequest)
            }
            Some(CaptureShowSingleTarget::ForwardedRequest) => {
                Some(CaptureShowTarget::ForwardedRequest)
            }
            Some(CaptureShowSingleTarget::UpstreamResponse) => {
                Some(CaptureShowTarget::UpstreamResponse)
            }
            Some(CaptureShowSingleTarget::OutboundResponse) => {
                Some(CaptureShowTarget::OutboundResponse)
            }
            None => None,
        });
        self.capture.render_query(&query)
    }

    #[tool(description = "List recent proxai capture records.")]
    fn capture_list(&self, Parameters(params): Parameters<CaptureListParams>) -> String {
        self.capture.render_query(&CaptureQuery::List(params.limit))
    }
}

#[tool_handler(
    name = "proxai",
    version = "0.2.0",
    instructions = "Use these tools to inspect and control proxai runtime capture behavior."
)]
impl ServerHandler for ProxaiMcpServer {}

pub async fn serve_http(
    listener: tokio::net::TcpListener,
    capture: CaptureController,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<()> {
    let service: StreamableHttpService<ProxaiMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(ProxaiMcpServer::new(capture.clone())),
            LocalSessionManager::default().into(),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new()
        .route("/health", get(health))
        .nest_service(MCP_PATH, service);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

pub fn endpoint_url(address: SocketAddr) -> String {
    format!("http://{address}{MCP_PATH}")
}

async fn health() -> &'static str {
    "OK"
}
