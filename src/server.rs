use axum::{
    Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
    routing::any,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::CorsLayer;
use tracing::{info, warn};

use crate::storage::AppState;
use crate::types::Webhook;

/// Build the webhook capture server
pub fn build_server(state: AppState) -> Router {
    Router::new()
        .route("/{*path}", any(handle_webhook))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Get the state file path from the app state
fn state_file_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let data_dir = std::path::PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("webtrap");
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("webtrap_state.json")
}

/// Handle any incoming webhook request
async fn handle_webhook(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    Query(_query_params): Query<HashMap<String, String>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Increment request counter
    {
        let mut count = state.stats.total_requests.write().await;
        *count += 1;
    }

    // Build header map
    let header_map: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or("<binary>").to_string(),
            )
        })
        .collect();

    // Get content type
    let content_type = header_map
        .get("content-type")
        .cloned()
        .unwrap_or_else(|| "application/octet-stream".to_string());

    // Parse body
    let raw_body = String::from_utf8_lossy(&body).to_string();
    let body_json: serde_json::Value =
        serde_json::from_slice(&body).unwrap_or(serde_json::Value::String(raw_body.clone()));

    // Build query string
    let query_string = uri.query().unwrap_or("").to_string();

    // Create webhook
    let webhook = Webhook::new(
        method.to_string(),
        uri.path().to_string(),
        query_string,
        header_map,
        body_json,
        Some(raw_body),
        content_type,
        addr.to_string(),
    );

    // Log the webhook
    info!(
        "Captured webhook: {} {} from {} ({} headers)",
        webhook.method,
        webhook.path,
        webhook.source_addr,
        webhook.headers.len()
    );

    // Store the webhook
    if state.config.store_captured {
        state.store.add(webhook.clone()).await;

        // Persist to state file
        let state_file = state_file_path();
        if let Ok(json) = state.store.export_json().await {
            tokio::fs::write(&state_file, json).await.ok();
        }
    }

    // Forward if configured
    if let Some(ref forward_url) = state.config.forward_url {
        let url = format!("{}{}", forward_url, uri.path());
        let mut forward_headers = HeaderMap::new();
        if state.config.forward_headers {
            forward_headers = headers.clone();
        }

        let client = reqwest::Client::new();
        let req_method = reqwest::Method::from_bytes(method.to_string().as_bytes())
            .unwrap_or(reqwest::Method::POST);
        let req = client
            .request(req_method, &url)
            .headers(forward_headers)
            .body(body.clone());
        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                info!("Forwarded webhook to {} — response: {}", url, status);
            }
            Err(e) => {
                warn!("Failed to forward webhook to {}: {}", url, e);
            }
        }
    }

    // Send response
    let status_code = StatusCode::from_u16(state.config.response_status).unwrap_or(StatusCode::OK);
    let response_headers = state.config.response_headers.clone();
    let response_body = state.config.response_body.clone();

    let mut resp = axum::response::Response::new(axum::body::Body::from(response_body));
    *resp.status_mut() = status_code;
    for (k, v) in response_headers {
        if let Ok(name) = axum::http::HeaderName::from_bytes(k.as_bytes())
            && let Ok(value) = axum::http::HeaderValue::from_str(&v)
        {
            resp.headers_mut().insert(name, value);
        }
    }
    resp
}
