use crate::{
    error::ConfigError,
    observe::DurationThresholds,
    protocol::{ProviderProtocol, RequestProtocol},
};
use serde::{Deserialize, Serialize};
use serde_with::{DurationSeconds, serde_as};
use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::path::PathBuf;
use std::time::Duration;
use strum::{Display, EnumString};
use url::Url;

const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: u16 = 18080;
const DEFAULT_MAX_REQUEST_BODY_BYTES: usize = 50 * 1024 * 1024;
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 64;
const DEFAULT_MCP_HOST: &str = "127.0.0.1";
const DEFAULT_MCP_PORT: u16 = 18081;
const DEFAULT_TOOL_CALLS_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub mcp: McpConfig,
    pub routing: RoutingConfig,
    pub providers: BTreeMap<String, ProviderConfig>,
    pub logging: LoggingConfig,
    pub error_responses: ErrorResponseConfig,
    pub tool_calls: ToolCallsConfig,
    pub capture: CaptureConfig,
}

impl AppConfig {
    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(&path).map_err(|source| ConfigError::Read {
            path: path.clone(),
            source,
        })?;
        let config =
            toml_edit::de::from_str::<Self>(&text).map_err(|source| ConfigError::Invalid {
                path: path.clone(),
                message: source.to_string(),
            })?;
        if config.server.max_request_body_bytes == 0 {
            return Err(ConfigError::Invalid {
                path: path.clone(),
                message: "server.max_request_body_bytes must be greater than 0".to_string(),
            });
        }
        if config.server.max_concurrent_requests == 0 {
            return Err(ConfigError::Invalid {
                path: path.clone(),
                message: "server.max_concurrent_requests must be greater than 0".to_string(),
            });
        }
        for (name, provider) in &config.providers {
            if provider.api_key.trim().is_empty() {
                return Err(ConfigError::Invalid {
                    path: path.clone(),
                    message: format!("providers.{name}.api_key must be a non-empty string"),
                });
            }
        }
        let mut route_names = BTreeSet::new();
        for (index, route) in config.routing.routes.iter().enumerate() {
            if let Some(name) = &route.name {
                let name = name.trim();
                if name.is_empty() {
                    return Err(ConfigError::Invalid {
                        path: path.clone(),
                        message: format!("routing.routes[{index}].name must be a non-empty string"),
                    });
                }
                if !route_names.insert(name.to_string()) {
                    return Err(ConfigError::Invalid {
                        path: path.clone(),
                        message: format!(
                            "routing.routes[{index}].name duplicates route name `{name}`"
                        ),
                    });
                }
            }
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub max_request_body_bytes: usize,
    pub max_concurrent_requests: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_SERVER_HOST.parse().expect("valid default host"),
            port: DEFAULT_SERVER_PORT,
            max_request_body_bytes: DEFAULT_MAX_REQUEST_BODY_BYTES,
            max_concurrent_requests: DEFAULT_MAX_CONCURRENT_REQUESTS,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct McpConfig {
    pub host: IpAddr,
    pub port: u16,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_MCP_HOST.parse().expect("valid default host"),
            port: DEFAULT_MCP_PORT,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoutingConfig {
    pub default_provider_names: DefaultProviderNamesConfig,
    pub routes: Vec<RouteConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DefaultProviderNamesConfig {
    pub openai_responses: String,
    pub openai_chat_completions: String,
    pub anthropic_messages: String,
}

pub(crate) fn normalize_provider_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum MatchKind {
    #[default]
    Auto,
    Exact,
    Glob,
    Regex,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RouteConfig {
    pub name: Option<String>,
    pub request_protocol: Option<RequestProtocol>,
    pub match_kind: MatchKind,
    pub model_pattern: String,
    pub provider: String,
    pub upstream_model: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub protocol: ProviderProtocol,
    pub base_url: Url,
    pub api_key: String,
    #[serde(default)]
    pub compatibility: ProviderCompatibility,
    #[serde_as(as = "DurationSeconds<u64>")]
    #[serde(rename = "read_idle_timeout_secs")]
    pub read_idle_timeout: Duration,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ProviderCompatibility {
    #[default]
    AnthropicCompatible,
    Strict,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub output_format: LogOutputFormat,
    pub use_color: bool,
    pub duration_thresholds: DurationThresholds,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            output_format: LogOutputFormat::default(),
            use_color: true,
            duration_thresholds: DurationThresholds::default(),
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum LogOutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, Display, EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case", ascii_case_insensitive)]
pub enum ErrorResponseFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ErrorResponseConfig {
    pub format: ErrorResponseFormat,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ToolCallsConfig {
    #[serde_as(as = "DurationSeconds<u64>")]
    #[serde(rename = "timeout_secs")]
    pub timeout: Duration,
}

impl Default for ToolCallsConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(DEFAULT_TOOL_CALLS_TIMEOUT_SECS),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CaptureConfig {
    pub inbound_request_enabled: bool,
    pub provider_request_enabled: bool,
    pub upstream_response_enabled: bool,
    pub outbound_response_enabled: bool,
}

impl CaptureConfig {
    pub fn any_enabled(self) -> bool {
        self.inbound_request_enabled
            || self.provider_request_enabled
            || self.upstream_response_enabled
            || self.outbound_response_enabled
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
