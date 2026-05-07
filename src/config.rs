use clap::Parser;
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(name = "hls-proxy", about = "HLS reverse proxy")]
pub struct Config {
    /// Address and port to listen on
    #[arg(long, env = "BIND", default_value = "0.0.0.0:8080")]
    pub bind: String,

    /// Public-facing base URL of this proxy (used to rewrite m3u8 URLs)
    #[arg(long, env = "BASE_URL")]
    pub base_url: String,

    /// Timeout in seconds for upstream requests
    #[arg(long, env = "UPSTREAM_TIMEOUT", default_value_t = 30)]
    pub upstream_timeout: u64,

    /// Log level
    #[arg(long, env = "LOG_LEVEL", default_value = "info")]
    pub log_level: String,
}

impl Config {
    pub fn upstream_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.upstream_timeout)
    }
}
