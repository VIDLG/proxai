use anyhow::Context;
use clap::{Args, Parser, Subcommand, ValueEnum};
use owo_colors::OwoColorize;
use proxai::{
    AppState,
    config::{AppConfig, LogLevel, LogOutputFormat, MatchKind, RouteConfig},
    logging, mcp, paths,
    protocol::RequestProtocol,
};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpListener;
use update_informer::{Check, registry};
use url::Url;

const GITHUB_REPOSITORY: &str = "VIDLG/proxai";
const RELEASES_URL: &str = "https://github.com/VIDLG/proxai/releases/latest";

#[derive(Debug, Clone, Args)]
struct RunArgs {
    /// Path to config.toml. If omitted, shim uses the standard app directory.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Override the default openai_responses provider base_url for this run.
    #[arg(long)]
    upstream: Option<String>,

    /// Override the default openai_responses provider api_key for this run.
    #[arg(long)]
    api_key: Option<String>,

    /// Temporary port override.
    #[arg(long)]
    port: Option<u16>,

    /// Temporary log level override.
    #[arg(long)]
    log_level: Option<String>,

    /// Temporary log format override: human or json.
    #[arg(long)]
    log_format: Option<String>,

    /// Temporarily override a named route field for this run.
    /// Format: route_name.field=value. Repeat to set multiple fields.
    /// Supported fields: request_protocol, match_kind, model_pattern, provider, upstream_model.
    #[arg(long = "route-override", value_name = "ROUTE.FIELD=VALUE")]
    route_overrides: Vec<String>,

    /// Temporarily enable inbound request capture for this run.
    #[arg(long)]
    capture_inbound_request: bool,

    /// Temporarily enable provider request capture for this run.
    #[arg(long)]
    capture_provider_request: bool,

    /// Temporarily enable upstream response capture for this run.
    #[arg(long)]
    capture_upstream_response: bool,

    /// Temporarily enable outbound response capture for this run.
    #[arg(long)]
    capture_outbound_response: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum CaptureTarget {
    InboundRequest,
    ProviderRequest,
    UpstreamResponse,
    OutboundResponse,
}

#[derive(Debug, Clone, Args)]
struct CaptureStatusArgs {
    /// Path to config.toml. If omitted, proxai uses the standard app directory.
    #[arg(long)]
    config: Option<PathBuf>,
}

#[derive(Debug, Clone, Args)]
struct CaptureToggleArgs {
    /// Path to config.toml. If omitted, proxai uses the standard app directory.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Which capture default to change. If omitted, affects all capture phases.
    #[arg(value_enum)]
    target: Option<CaptureTarget>,
}

#[derive(Debug, Clone, Subcommand)]
enum CaptureCommand {
    /// Show the configured default capture settings.
    Status(CaptureStatusArgs),

    /// Enable capture defaults in config.toml.
    Enable(CaptureToggleArgs),

    /// Disable capture defaults in config.toml.
    Disable(CaptureToggleArgs),
}

#[derive(Debug, Clone, Subcommand)]
enum Command {
    /// Check GitHub Releases for a newer build.
    #[command(name = "check-update")]
    CheckUpdate,

    /// Inspect and update capture defaults.
    Capture {
        #[command(subcommand)]
        command: CaptureCommand,
    },
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "proxai",
    version,
    about = "Run a local async compatibility proxy for Zed and OpenAI-compatible clients."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    run: RunArgs,
}

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::CheckUpdate) => return check_update(),
        Some(Command::Capture { command }) => return run_capture_command(command),
        None => {}
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build Tokio runtime")?;
    runtime.block_on(run(cli.run))
}

fn run_capture_command(command: CaptureCommand) -> anyhow::Result<()> {
    match command {
        CaptureCommand::Status(args) => capture_status(args),
        CaptureCommand::Enable(args) => set_capture_defaults(args, true),
        CaptureCommand::Disable(args) => set_capture_defaults(args, false),
    }
}

fn capture_status(args: CaptureStatusArgs) -> anyhow::Result<()> {
    let app_paths = paths::ensure_app_paths().context("prepare app paths")?;
    let active_config_path = args
        .config
        .clone()
        .unwrap_or_else(|| app_paths.config_path.clone());
    let config = AppConfig::load(active_config_path.clone())?;

    println!("capture");
    println!("  config: {}", active_config_path.display());
    println!(
        "  inbound_request_enabled: {}",
        config.capture.inbound_request_enabled
    );
    println!(
        "  provider_request_enabled: {}",
        config.capture.provider_request_enabled
    );
    println!(
        "  upstream_response_enabled: {}",
        config.capture.upstream_response_enabled
    );
    println!(
        "  outbound_response_enabled: {}",
        config.capture.outbound_response_enabled
    );
    println!("  captures_dir: {}", app_paths.captures_dir.display());

    Ok(())
}

fn set_capture_defaults(args: CaptureToggleArgs, enabled: bool) -> anyhow::Result<()> {
    let app_paths = paths::ensure_app_paths().context("prepare app paths")?;
    let active_config_path = args
        .config
        .clone()
        .unwrap_or_else(|| app_paths.config_path.clone());

    let text = std::fs::read_to_string(&active_config_path)
        .with_context(|| format!("read {}", active_config_path.display()))?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("parse {}", active_config_path.display()))?;

    let capture = doc["capture"].or_insert(toml_edit::table());

    match args.target {
        Some(CaptureTarget::InboundRequest) => {
            capture["inbound_request_enabled"] = toml_edit::value(enabled);
        }
        Some(CaptureTarget::ProviderRequest) => {
            capture["provider_request_enabled"] = toml_edit::value(enabled);
        }
        Some(CaptureTarget::UpstreamResponse) => {
            capture["upstream_response_enabled"] = toml_edit::value(enabled);
        }
        Some(CaptureTarget::OutboundResponse) => {
            capture["outbound_response_enabled"] = toml_edit::value(enabled);
        }
        None => {
            capture["inbound_request_enabled"] = toml_edit::value(enabled);
            capture["provider_request_enabled"] = toml_edit::value(enabled);
            capture["upstream_response_enabled"] = toml_edit::value(enabled);
            capture["outbound_response_enabled"] = toml_edit::value(enabled);
        }
    }

    std::fs::write(&active_config_path, doc.to_string())
        .with_context(|| format!("write {}", active_config_path.display()))?;

    println!("capture");
    println!("  config: {}", active_config_path.display());
    println!(
        "  inbound_request_enabled: {}",
        match args.target {
            Some(CaptureTarget::ProviderRequest)
            | Some(CaptureTarget::UpstreamResponse)
            | Some(CaptureTarget::OutboundResponse) => "unchanged".to_string(),
            _ => enabled.to_string(),
        }
    );
    println!(
        "  provider_request_enabled: {}",
        match args.target {
            Some(CaptureTarget::InboundRequest)
            | Some(CaptureTarget::UpstreamResponse)
            | Some(CaptureTarget::OutboundResponse) => "unchanged".to_string(),
            _ => enabled.to_string(),
        }
    );
    println!(
        "  upstream_response_enabled: {}",
        match args.target {
            Some(CaptureTarget::InboundRequest)
            | Some(CaptureTarget::ProviderRequest)
            | Some(CaptureTarget::OutboundResponse) => "unchanged".to_string(),
            _ => enabled.to_string(),
        }
    );
    println!(
        "  outbound_response_enabled: {}",
        match args.target {
            Some(CaptureTarget::InboundRequest)
            | Some(CaptureTarget::ProviderRequest)
            | Some(CaptureTarget::UpstreamResponse) => "unchanged".to_string(),
            _ => enabled.to_string(),
        }
    );

    Ok(())
}

async fn run(cli: RunArgs) -> anyhow::Result<()> {
    let app_paths = paths::ensure_app_paths().context("prepare app paths")?;
    let active_config_path = cli
        .config
        .clone()
        .unwrap_or_else(|| app_paths.config_path.clone());

    let mut config = AppConfig::load(active_config_path.clone())?;
    if config
        .routing
        .default_provider_names
        .openai_responses
        .trim()
        .is_empty()
    {
        anyhow::bail!(
            "routing.default_provider_names.openai_responses is required in {}",
            active_config_path.display()
        );
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    if let Some(log_level) = cli.log_level {
        config.logging.level = LogLevel::from_str(&log_level)
            .with_context(|| format!("invalid --log-level {log_level:?}"))?;
    }
    if let Some(log_format) = cli.log_format {
        config.logging.output_format = LogOutputFormat::from_str(&log_format)
            .with_context(|| format!("invalid --log-format {log_format:?}"))?;
    }
    apply_route_overrides(&mut config.routing.routes, &cli.route_overrides)?;

    if cli.capture_inbound_request {
        config.capture.inbound_request_enabled = true;
    }
    if cli.capture_provider_request {
        config.capture.provider_request_enabled = true;
    }
    if cli.capture_upstream_response {
        config.capture.upstream_response_enabled = true;
    }
    if cli.capture_outbound_response {
        config.capture.outbound_response_enabled = true;
    }

    let default_provider_name = config
        .routing
        .default_provider_names
        .openai_responses
        .trim()
        .to_ascii_lowercase();
    let default_provider = config
        .providers
        .remove(&default_provider_name)
        .or_else(|| {
            config
                .providers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&default_provider_name))
                .map(|(name, _)| name.clone())
                .and_then(|name| config.providers.remove(&name))
        })
        .with_context(|| {
            format!(
                "default provider {:?} is missing from [providers.*] in {}",
                config.routing.default_provider_names.openai_responses,
                active_config_path.display()
            )
        })?;

    let upstream = cli
        .upstream
        .and_then(|value| normalize_config_value(Some(value)))
        .map(|value| Url::parse(&value))
        .transpose()
        .context("parse --upstream")?
        .unwrap_or_else(|| default_provider.base_url.clone());
    let api_key = cli
        .api_key
        .and_then(|value| normalize_config_value(Some(value)))
        .unwrap_or_else(|| default_provider.api_key.clone());

    logging::init(
        config.logging.level,
        config.logging.output_format,
        config.logging.use_color,
        config.logging.duration_thresholds.clone(),
    );

    let address = SocketAddr::new(config.server.host, config.server.port);
    let mut providers = BTreeMap::new();
    providers.insert(
        default_provider_name.clone(),
        proxai::config::ProviderConfig {
            base_url: upstream.clone(),
            api_key: api_key.clone(),
            ..default_provider
        },
    );
    for (name, provider) in config.providers {
        providers.insert(name.to_ascii_lowercase(), provider);
    }

    let default_provider_names = proxai::config::DefaultProviderNamesConfig {
        openai_responses: default_provider_name.clone(),
        openai_chat_completions: config
            .routing
            .default_provider_names
            .openai_chat_completions
            .clone(),
        anthropic_messages: config
            .routing
            .default_provider_names
            .anthropic_messages
            .clone(),
    };

    let state = AppState::new(
        default_provider_names,
        providers,
        config.routing.routes.clone(),
    )
    .context("parse upstream URL")?
    .with_server_limits(
        config.server.max_request_body_bytes,
        config.server.max_concurrent_requests,
    )
    .with_error_response_format(config.error_responses.format)
    .with_capture_dir(Some(app_paths.captures_dir.clone()))
    .with_capture_config(config.capture)
    .with_sse_tool_call_timeout(Some(config.tool_calls.timeout));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("bind {address}"))?;
    let mcp_address = SocketAddr::new(config.mcp.host, config.mcp.port);
    let mcp_listener = TcpListener::bind(mcp_address)
        .await
        .with_context(|| format!("bind {mcp_address}"))?;
    let capture = state.capture_controller();

    let startup_color =
        matches!(config.logging.output_format, LogOutputFormat::Human) && config.logging.use_color;

    println!("{}", startup_title(startup_color, "ProxAI"));
    println!(
        "  {} {}",
        startup_label(startup_color, "listen:"),
        startup_url(startup_color, &format!("http://{address}"))
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "mcp:"),
        startup_url(startup_color, &mcp::endpoint_url(mcp_address))
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "max_request_body_bytes:"),
        startup_value(
            startup_color,
            &config.server.max_request_body_bytes.to_string()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "max_concurrent_requests:"),
        startup_value(
            startup_color,
            &config.server.max_concurrent_requests.to_string()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "default_provider.responses:"),
        startup_value(startup_color, &default_provider_name)
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "default_provider.chat:"),
        startup_value(
            startup_color,
            config
                .routing
                .default_provider_names
                .openai_chat_completions
                .trim()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "default_provider.anthropic:"),
        startup_value(
            startup_color,
            config
                .routing
                .default_provider_names
                .anthropic_messages
                .trim()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "default_upstream:"),
        startup_url(startup_color, upstream.as_str().trim_end_matches('/'))
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "app_dir:"),
        startup_path(startup_color, &app_paths.app_dir.display().to_string())
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "config:"),
        startup_path(startup_color, &active_config_path.display().to_string())
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "config_example:"),
        startup_path(
            startup_color,
            &app_paths.config_example_path.display().to_string()
        )
    );
    if app_paths.created_config {
        println!(
            "  {} {}",
            startup_label(startup_color, "note:"),
            startup_note(
                startup_color,
                "config.toml was created from the example template; set [providers.<name>].base_url, [providers.<name>].api_key, and routing before use"
            )
        );
    }
    let tool_alias_lines = startup_tool_alias_lines(startup_color);
    if let Some((first, rest)) = tool_alias_lines.split_first() {
        println!(
            "  {} {}",
            startup_label(startup_color, "tool_aliases:"),
            first
        );
        let alias_indent = " ".repeat(2 + "tool_aliases:".len() + 1);
        for line in rest {
            println!("{alias_indent}{line}");
        }
    }
    let request_hint_lines = startup_request_hint_lines(startup_color);
    if let Some((first, rest)) = request_hint_lines.split_first() {
        println!(
            "  {} {}",
            startup_label(startup_color, "request_hints:"),
            first
        );
        let hint_indent = " ".repeat(2 + "request_hints:".len() + 1);
        for line in rest {
            println!("{hint_indent}{line}");
        }
    }
    println!(
        "  {} {}",
        startup_label(startup_color, "log_level:"),
        startup_value(startup_color, &config.logging.level.to_string())
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "log_format:"),
        startup_value(startup_color, &config.logging.output_format.to_string())
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "log_use_color:"),
        startup_bool(startup_color, config.logging.use_color)
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "error_response_format:"),
        startup_value(
            startup_color,
            &format!("{:?}", config.error_responses.format)
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "route_count:"),
        startup_value(startup_color, &config.routing.routes.len().to_string())
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "read_idle_timeout_secs:"),
        startup_value(
            startup_color,
            &default_provider.read_idle_timeout.as_secs().to_string()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "tool_calls.timeout_secs:"),
        startup_value(
            startup_color,
            &config.tool_calls.timeout.as_secs().to_string()
        )
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "capture.inbound_request_enabled:"),
        startup_bool(startup_color, config.capture.inbound_request_enabled)
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "capture.provider_request_enabled:"),
        startup_bool(startup_color, config.capture.provider_request_enabled)
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "capture.upstream_response_enabled:"),
        startup_bool(startup_color, config.capture.upstream_response_enabled)
    );
    println!(
        "  {} {}",
        startup_label(startup_color, "capture.outbound_response_enabled:"),
        startup_bool(startup_color, config.capture.outbound_response_enabled)
    );
    if config.capture.any_enabled() {
        println!(
            "  {} {}",
            startup_label(startup_color, "capture:"),
            startup_path(startup_color, &app_paths.captures_dir.display().to_string())
        );
    } else {
        println!(
            "  {} {}",
            startup_label(startup_color, "capture:"),
            startup_note(startup_color, "disabled")
        );
    }

    tokio::try_join!(
        state.serve(listener, shutdown_signal()),
        mcp::serve_http(mcp_listener, capture, shutdown_signal())
    )
    .context("run local servers")?;
    Ok(())
}

fn check_update() -> anyhow::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::GitHub, GITHUB_REPOSITORY, current_version)
        .interval(Duration::ZERO)
        .timeout(Duration::from_secs(5));

    println!("current: {current_version}");
    match informer
        .check_version()
        .map_err(|error| anyhow::anyhow!("check latest GitHub release: {error}"))?
    {
        Some(version) => {
            println!("latest:  {version}");
            println!("status:  update available");
            println!("release: {RELEASES_URL}");
        }
        None => {
            println!("status:  up to date");
            println!("release: {RELEASES_URL}");
        }
    }

    Ok(())
}

fn normalize_config_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn apply_route_overrides(routes: &mut [RouteConfig], overrides: &[String]) -> anyhow::Result<()> {
    for override_spec in overrides {
        let (selector, value) = override_spec.split_once('=').with_context(|| {
            format!("invalid --route-override {override_spec:?}; expected ROUTE.FIELD=VALUE")
        })?;
        let (route_name, field_name) = selector.split_once('.').with_context(|| {
            format!("invalid --route-override {override_spec:?}; expected ROUTE.FIELD=VALUE")
        })?;
        let route_name = route_name.trim();
        let field_name = field_name.trim();
        if route_name.is_empty() || field_name.is_empty() {
            anyhow::bail!(
                "invalid --route-override {override_spec:?}; route name and field must be non-empty"
            );
        }

        let route = find_route_by_name(routes, route_name)
            .with_context(|| format!("apply --route-override {override_spec:?}"))?;
        apply_route_override_field(route, field_name, value.trim())
            .with_context(|| format!("apply --route-override {override_spec:?}"))?;
    }
    Ok(())
}

fn find_route_by_name<'a>(
    routes: &'a mut [RouteConfig],
    route_name: &str,
) -> anyhow::Result<&'a mut RouteConfig> {
    let mut matches = routes
        .iter_mut()
        .filter(|route| route.name.as_deref() == Some(route_name));
    let Some(route) = matches.next() else {
        anyhow::bail!("unknown route name `{route_name}`");
    };
    if matches.next().is_some() {
        anyhow::bail!("route name `{route_name}` is not unique");
    }
    Ok(route)
}

fn apply_route_override_field(
    route: &mut RouteConfig,
    field_name: &str,
    value: &str,
) -> anyhow::Result<()> {
    match field_name {
        "request_protocol" => {
            route.request_protocol = if value.is_empty() {
                None
            } else {
                Some(
                    RequestProtocol::from_str(value)
                        .with_context(|| format!("invalid request_protocol {value:?}"))?,
                )
            };
        }
        "match_kind" => {
            route.match_kind = MatchKind::from_str(value)
                .with_context(|| format!("invalid match_kind {value:?}"))?;
        }
        "model_pattern" => route.model_pattern = value.to_string(),
        "provider" => route.provider = value.to_string(),
        "upstream_model" => route.upstream_model = normalize_config_value(Some(value.to_string())),
        "name" => anyhow::bail!("route name cannot be overridden at runtime"),
        other => anyhow::bail!(
            "unsupported route override field `{other}`; supported fields: request_protocol, match_kind, model_pattern, provider, upstream_model"
        ),
    }
    Ok(())
}

fn startup_title(color: bool, value: &str) -> String {
    if color {
        value.bright_blue().bold().to_string()
    } else {
        value.to_string()
    }
}

fn startup_label(color: bool, value: &str) -> String {
    if color {
        value.bright_black().bold().to_string()
    } else {
        value.to_string()
    }
}

fn startup_url(color: bool, value: &str) -> String {
    if color {
        value.cyan().to_string()
    } else {
        value.to_string()
    }
}

fn startup_path(color: bool, value: &str) -> String {
    if color {
        value.yellow().to_string()
    } else {
        value.to_string()
    }
}

fn startup_note(color: bool, value: &str) -> String {
    if color {
        value.bright_black().to_string()
    } else {
        value.to_string()
    }
}

fn startup_tool_alias_lines(color: bool) -> Vec<String> {
    const PER_LINE: usize = 3;
    let entries = proxai::TOOL_NAME_ALIASES
        .iter()
        .map(|(alias, full)| {
            if color {
                format!(
                    "{}{}{}",
                    alias.bright_blue().bold(),
                    "=".bright_black(),
                    full.cyan()
                )
            } else {
                format!("{alias}={full}")
            }
        })
        .collect::<Vec<_>>();

    entries
        .chunks(PER_LINE)
        .map(|chunk| chunk.join("  "))
        .collect()
}

fn startup_request_hint_lines(color: bool) -> Vec<String> {
    const PER_LINE: usize = 3;
    const ENTRIES: [(&str, &str); 12] = [
        ("ptc", "parallel_tool_calls"),
        ("pck", "prompt_cache_key"),
        ("instr", "instructions"),
        ("include[...]", "include fields"),
        ("rsn", "reasoning"),
        ("rsn.enc", "reasoning.encrypted_content"),
        ("tc:*", "tool_choice"),
        ("tools[...]", "tool inventory"),
        ("mtc", "max_tool_calls"),
        ("pcr", "prompt_cache_retention"),
        ("rs", "reasoning.summary"),
        ("tlp", "top_logprobs"),
    ];

    ENTRIES
        .iter()
        .map(|(alias, full)| {
            if color {
                format!(
                    "{}{}{}",
                    alias.bright_blue().bold(),
                    "=".bright_black(),
                    full.cyan()
                )
            } else {
                format!("{alias}={full}")
            }
        })
        .collect::<Vec<_>>()
        .chunks(PER_LINE)
        .map(|chunk| chunk.join("  "))
        .collect()
}

fn startup_value(color: bool, value: &str) -> String {
    if color {
        value.green().to_string()
    } else {
        value.to_string()
    }
}

fn startup_bool(color: bool, value: bool) -> String {
    if color {
        if value {
            "true".green().bold().to_string()
        } else {
            "false".red().bold().to_string()
        }
    } else {
        value.to_string()
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };

    #[cfg(windows)]
    let terminate = async {
        let mut signal = tokio::signal::windows::ctrl_break().expect("install Ctrl+Break handler");
        signal.recv().await;
    };

    #[cfg(not(windows))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
