use std::sync::Arc;
use tokio::sync::RwLock;
use crate::types::Webhook;

/// Thread-safe webhook storage
#[derive(Debug, Clone)]
pub struct WebhookStore {
    inner: Arc<RwLock<Vec<Webhook>>>,
}

impl WebhookStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a webhook to the store (async)
    pub async fn add(&self, webhook: Webhook) {
        let mut store = self.inner.write().await;
        store.push(webhook);
    }

    /// Add a webhook synchronously (for testing)
    pub fn add_sync(&self, webhook: Webhook) {
        let mut store = self.inner.blocking_write();
        store.push(webhook);
    }

    /// Get all webhooks (async)
    pub async fn all(&self) -> Vec<Webhook> {
        let store = self.inner.read().await;
        store.clone()
    }

    /// Get all webhooks synchronously (for testing)
    pub fn all_sync(&self) -> Vec<Webhook> {
        let store = self.inner.blocking_read();
        store.clone()
    }

    /// Get a webhook by ID
    pub async fn get(&self, id: &str) -> Option<Webhook> {
        let store = self.inner.read().await;
        store.iter().find(|w| w.id.to_string() == id).cloned()
    }

    /// Get a webhook by ID synchronously (for testing)
    pub fn get_sync(&self, id: &str) -> Option<Webhook> {
        let store = self.inner.blocking_read();
        store.iter().find(|w| w.id.to_string() == id).cloned()
    }

    /// Get the most recent webhooks
    pub async fn recent(&self, count: usize) -> Vec<Webhook> {
        let store = self.inner.read().await;
        store.iter().rev().take(count).cloned().collect()
    }

    /// Get the most recent webhooks synchronously (for testing)
    pub fn recent_sync(&self, count: usize) -> Vec<Webhook> {
        let store = self.inner.blocking_read();
        store.iter().rev().take(count).cloned().collect()
    }

    /// Clear all webhooks
    pub async fn clear(&self) {
        let mut store = self.inner.write().await;
        store.clear();
    }

    /// Clear all webhooks synchronously (for testing)
    pub fn clear_sync(&self) {
        let mut store = self.inner.blocking_write();
        store.clear();
    }

    /// Count webhooks
    pub async fn count(&self) -> usize {
        let store = self.inner.read().await;
        store.len()
    }

    /// Count webhooks synchronously (for testing)
    pub fn count_sync(&self) -> usize {
        let store = self.inner.blocking_read();
        store.len()
    }

    /// Tag a webhook
    pub async fn tag(&self, id: &str, tags: Vec<String>) -> bool {
        let mut store = self.inner.write().await;
        if let Some(webhook) = store.iter_mut().find(|w| w.id.to_string() == id) {
            webhook.tags = tags;
            true
        } else {
            false
        }
    }

    /// Tag a webhook synchronously (for testing)
    pub fn tag_sync(&self, id: &str, tags: Vec<String>) -> bool {
        let mut store = self.inner.blocking_write();
        if let Some(webhook) = store.iter_mut().find(|w| w.id.to_string() == id) {
            webhook.tags = tags;
            true
        } else {
            false
        }
    }

    /// Export webhooks as JSON
    pub async fn export_json(&self) -> Result<String, serde_json::Error> {
        let store = self.inner.read().await;
        serde_json::to_string_pretty(&*store)
    }

    /// Export webhooks as JSON synchronously (for testing)
    pub fn export_json_sync(&self) -> Result<String, serde_json::Error> {
        let store = self.inner.blocking_read();
        serde_json::to_string_pretty(&*store)
    }
}

/// Shared application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub store: WebhookStore,
    pub config: crate::types::ServerConfig,
    pub stats: Stats,
}

#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub total_requests: Arc<RwLock<u64>>,
    pub active_connections: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new(config: crate::types::ServerConfig) -> Self {
        Self {
            store: WebhookStore::new(),
            config,
            stats: Stats::default(),
        }
    }
}