use anyhow::Context;
use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use zed_openai_shim::AppState;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "zed-openai-shim",
    version,
    about = "Run a local async Zed OpenAI compatibility proxy."
)]
struct Config {
    #[arg(long, env = "OPENAI_SHIM_UPSTREAM")]
    upstream: String,

    #[arg(long, env = "OPENAI_SHIM_HOST", default_value = "127.0.0.1")]
    host: IpAddr,

    #[arg(long, env = "OPENAI_SHIM_PORT", default_value_t = 18080)]
    port: u16,

    #[arg(long, env = "OPENAI_SHIM_TIMEOUT_SECONDS", default_value_t = 300)]
    timeout_seconds: u64,

    #[arg(long, env = "OPENAI_SHIM_LOG_LEVEL", default_value = "info")]
    log_level: String,

    #[arg(long, env = "OPENAI_SHIM_API_KEY")]
    api_key: Option<String>,

    #[arg(
        long,
        env = "OPENAI_SHIM_OVERRIDE_AUTHORIZATION",
        default_value_t = false
    )]
    override_authorization: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dotenvy::from_path(dir.join(".env")).ok();
        }
    }
    if let Some(home) = home::home_dir() {
        dotenvy::from_path(home.join(".zed-openai-shim").join(".env")).ok();
    }
    dotenvy::dotenv().ok();

    let config = Config::parse();
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let address = SocketAddr::new(config.host, config.port);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout_seconds))
        .build()
        .context("build upstream client")?;
    let state = AppState::new(
        config.upstream.clone(),
        config.api_key.clone(),
        config.override_authorization,
        client,
    )
    .context("parse upstream URL")?;
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("bind {address}"))?;

    println!(
        "Zed OpenAI shim listening on http://{}; upstream={}",
        address,
        config.upstream.trim_end_matches('/')
    );

    state
        .serve(listener, shutdown_signal())
        .await
        .context("run server")
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
