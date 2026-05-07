use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{debug, error, instrument, warn};
use url::Url;

use crate::rewrite::rewrite_playlist;

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub base_url: Arc<String>,
}

#[derive(Deserialize, Debug)]
pub struct ProxyParams {
    pub url: String,
}

#[instrument(skip(state), fields(upstream = %params.url))]
pub async fn proxy_handler(
    State(state): State<AppState>,
    Query(params): Query<ProxyParams>,
) -> Response {
    let upstream_url = match Url::parse(&params.url) {
        Ok(u) => u,
        Err(e) => {
            warn!("invalid upstream URL: {e}");
            return (StatusCode::BAD_REQUEST, format!("invalid url: {e}")).into_response();
        }
    };

    debug!("fetching upstream");

    let upstream_resp = match state.client.get(upstream_url.as_str()).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("upstream request failed: {e}");
            return (StatusCode::BAD_GATEWAY, format!("upstream error: {e}")).into_response();
        }
    };

    let status = upstream_resp.status();
    let content_type = upstream_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if !status.is_success() {
        warn!("upstream returned {status}");
        return StatusCode::from_u16(status.as_u16())
            .unwrap_or(StatusCode::BAD_GATEWAY)
            .into_response();
    }

    if is_playlist(&content_type, upstream_url.as_str()) {
        handle_playlist(upstream_resp, upstream_url, &state.base_url, content_type).await
    } else {
        stream_bytes(upstream_resp, content_type).await
    }
}

fn is_playlist(content_type: &str, url: &str) -> bool {
    content_type.contains("mpegurl")
        || content_type.contains("x-mpegurl")
        || url.ends_with(".m3u8")
        || url.ends_with(".m3u")
}

async fn handle_playlist(
    resp: reqwest::Response,
    upstream_url: Url,
    proxy_base: &str,
    content_type: String,
) -> Response {
    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("failed to read playlist body: {e}");
            return (StatusCode::BAD_GATEWAY, "failed to read upstream body").into_response();
        }
    };

    let rewritten = rewrite_playlist(&body, &upstream_url, proxy_base);

    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&content_type) {
        headers.insert(axum::http::header::CONTENT_TYPE, v);
    }
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );

    (StatusCode::OK, headers, rewritten).into_response()
}

async fn stream_bytes(resp: reqwest::Response, content_type: String) -> Response {
    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&content_type) {
        headers.insert(axum::http::header::CONTENT_TYPE, v);
    }

    // Forward Content-Length if present so clients can show progress
    if let Some(len) = resp.content_length() {
        if let Ok(v) = HeaderValue::from_str(&len.to_string()) {
            headers.insert(axum::http::header::CONTENT_LENGTH, v);
        }
    }

    let stream = resp.bytes_stream();
    let body = Body::from_stream(stream);

    (StatusCode::OK, headers, body).into_response()
}

// ------------------------------------------------------------------
// Convert reqwest::Response bytes_stream item type for axum Body
// ------------------------------------------------------------------
// reqwest streams yield Result<Bytes, reqwest::Error>; axum Body::from_stream
// requires the error to implement Into<Box<dyn std::error::Error + Send + Sync>>,
// which reqwest::Error already satisfies.

// Ensure Bytes is in scope for the stream conversion
#[allow(dead_code)]
fn _bytes_check(_: Bytes) {}
