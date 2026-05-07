mod config;
mod proxy;
mod rewrite;

use std::sync::Arc;

use axum::{Router, routing::get};
use clap::Parser;
use reqwest::Client;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};

use config::Config;
use proxy::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::parse();

    fmt()
        .with_env_filter(
            EnvFilter::try_new(&cfg.log_level)
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let client = Client::builder()
        .timeout(cfg.upstream_timeout_duration())
        .build()?;

    let state = AppState {
        client,
        base_url: Arc::new(cfg.base_url.clone()),
    };

    let app = Router::new()
        .route("/proxy", get(proxy::proxy_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&cfg.bind).await?;
    info!(bind = %cfg.bind, base_url = %cfg.base_url, "hls-proxy listening");

    axum::serve(listener, app).await?;

    Ok(())
}
