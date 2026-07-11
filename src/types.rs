use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A captured webhook request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: Uuid,
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
    pub raw_body: Option<String>,
    pub content_type: String,
    pub source_addr: String,
    pub received_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Webhook {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        method: String,
        path: String,
        query: String,
        headers: HashMap<String, String>,
        body: serde_json::Value,
        raw_body: Option<String>,
        content_type: String,
        source_addr: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            method,
            path,
            query,
            headers,
            body,
            raw_body,
            content_type,
            source_addr,
            received_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Format the webhook for display
    pub fn summary(&self) -> String {
        let body_str = if self.body.is_object() || self.body.is_array() {
            serde_json::to_string_pretty(&self.body).unwrap_or_default()
        } else {
            self.body.to_string()
        };
        let body_preview = if body_str.len() > 200 {
            format!("{}...", &body_str[..200])
        } else {
            body_str
        };
        let tags_display = if self.tags.is_empty() {
            "none".to_string()
        } else {
            self.tags.join(", ")
        };
        format!(
            "┌─ {} {} ──────────────────────────────────\n\
             │ ID:        {}\n\
             │ Time:      {}\n\
             │ From:      {}\n\
             │ Type:      {}\n\
             │ Tags:      {}\n\
             │ Headers:   {} headers\n\
             │ Body:      {}\n\
             └──────────────────────────────────────────────",
            self.method,
            self.path,
            self.id,
            self.received_at.format("%H:%M:%S%.3f"),
            self.source_addr,
            self.content_type,
            tags_display,
            self.headers.len(),
            body_preview,
        )
    }
}

/// CLI output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Pretty,
}

/// Signature validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureResult {
    pub webhook_id: Uuid,
    pub provider: String,
    pub signature_header: String,
    pub signature_value: String,
    pub computed_signature: String,
    pub valid: bool,
    pub details: String,
}

/// Filter criteria for webhooks
#[derive(Debug, Clone, Default)]
pub struct WebhookFilter {
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub tag: Option<String>,
    pub source: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_body_size: usize,
    pub response_status: u16,
    pub response_body: String,
    pub response_headers: HashMap<String, String>,
    pub store_captured: bool,
    pub forward_url: Option<String>,
    pub forward_headers: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_body_size: 1024 * 1024, // 1MB
            response_status: 200,
            response_body: "OK".to_string(),
            response_headers: HashMap::new(),
            store_captured: true,
            forward_url: None,
            forward_headers: true,
        }
    }
}
