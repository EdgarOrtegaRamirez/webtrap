use std::collections::HashMap;

use webtrap::types::{Webhook, WebhookFilter, OutputFormat, SignatureResult, ServerConfig};
use webtrap::storage::{WebhookStore, AppState};

#[test]
fn test_webhook_creation() {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("x-hub-signature-256".to_string(), "sha256=abc123".to_string());

    let body = serde_json::json!({"event": "push", "ref": "refs/heads/main"});

    let wh = Webhook::new(
        "POST".to_string(),
        "/webhooks/github".to_string(),
        "token=abc".to_string(),
        headers.clone(),
        body.clone(),
        Some(r#"{"event":"push","ref":"refs/heads/main"}"#.to_string()),
        "application/json".to_string(),
        "192.168.1.1:54321".to_string(),
    );

    assert_eq!(wh.method, "POST");
    assert_eq!(wh.path, "/webhooks/github");
    assert_eq!(wh.query, "token=abc");
    assert_eq!(wh.headers.len(), 2);
    assert_eq!(wh.body, body);
    assert_eq!(wh.content_type, "application/json");
    assert_eq!(wh.source_addr, "192.168.1.1:54321");
    assert!(wh.tags.is_empty());
    assert!(wh.raw_body.is_some());
}

#[test]
fn test_webhook_summary() {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "text/plain".to_string());

    let wh = Webhook::new(
        "GET".to_string(),
        "/test".to_string(),
        "".to_string(),
        headers,
        serde_json::Value::String("Hello".to_string()),
        Some("Hello".to_string()),
        "text/plain".to_string(),
        "127.0.0.1:12345".to_string(),
    );

    let summary = wh.summary();
    assert!(summary.contains("GET"));
    assert!(summary.contains("/test"));
    assert!(summary.contains("127.0.0.1"));
    assert!(summary.contains("Hello"));
}

#[test]
fn test_webhook_store_add_get() {
    let store = WebhookStore::new();
    let wh = create_test_webhook("POST", "/hook", "test body");

    assert_eq!(store.count_sync(), 0);
    store.add_sync(wh.clone());
    assert_eq!(store.count_sync(), 1);

    let all = store.all_sync();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].method, "POST");

    let id_str = all[0].id.to_string();
    let fetched = store.get_sync(&id_str);
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().path, "/hook");
}

#[test]
fn test_webhook_store_clear() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/a", "body1"));
    store.add_sync(create_test_webhook("POST", "/b", "body2"));
    assert_eq!(store.count_sync(), 2);

    store.clear_sync();
    assert_eq!(store.count_sync(), 0);
}

#[test]
fn test_webhook_store_recent() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/1", "first"));
    store.add_sync(create_test_webhook("POST", "/2", "second"));
    store.add_sync(create_test_webhook("POST", "/3", "third"));

    let recent = store.recent_sync(2);
    assert_eq!(recent.len(), 2);
    // Most recent first
    assert_eq!(recent[0].path, "/3");
    assert_eq!(recent[1].path, "/2");
}

#[test]
fn test_webhook_store_tag() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/hook", "body"));

    let all = store.all_sync();
    let id = all[0].id.to_string();

    let result = store.tag_sync(&id, vec!["important".to_string(), "test".to_string()]);
    assert!(result);

    let fetched = store.get_sync(&id).unwrap();
    assert_eq!(fetched.tags, vec!["important", "test"]);
}

#[test]
fn test_webhook_store_tag_not_found() {
    let store = WebhookStore::new();
    let result = store.tag_sync("nonexistent-id", vec!["tag".to_string()]);
    assert!(!result);
}

#[test]
fn test_webhook_store_export_json() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/hook", r#"{"key":"value"}"#));

    let json = store.export_json_sync().unwrap();
    assert!(json.contains("POST"));
    assert!(json.contains("/hook"));
}

#[test]
fn test_webhook_filter_method() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/post-hook", "post body"));
    store.add_sync(create_test_webhook("GET", "/get-hook", "get body"));

    let filter = WebhookFilter {
        method: Some("GET".to_string()),
        ..Default::default()
    };

    let webhooks = store.all_sync();
    let filtered: Vec<_> = webhooks.iter()
        .filter(|w| {
            if let Some(ref m) = filter.method {
                w.method.eq_ignore_ascii_case(m)
            } else {
                true
            }
        })
        .cloned()
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].method, "GET");
}

#[test]
fn test_webhook_filter_path() {
    let store = WebhookStore::new();
    store.add_sync(create_test_webhook("POST", "/api/v1/hooks", "body1"));
    store.add_sync(create_test_webhook("POST", "/api/v2/hooks", "body2"));
    store.add_sync(create_test_webhook("POST", "/other", "body3"));

    let filter = WebhookFilter {
        path: Some("api/v1".to_string()),
        ..Default::default()
    };

    let webhooks = store.all_sync();
    let filtered: Vec<_> = webhooks.iter()
        .filter(|w| {
            if let Some(ref p) = filter.path {
                w.path.contains(p.as_str())
            } else {
                true
            }
        })
        .cloned()
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].path, "/api/v1/hooks");
}

#[test]
fn test_signature_result_serialization() {
    let result = SignatureResult {
        webhook_id: uuid::Uuid::new_v4(),
        provider: "github".to_string(),
        signature_header: "x-hub-signature-256".to_string(),
        signature_value: "sha256=abc123".to_string(),
        computed_signature: "sha256=abc123".to_string(),
        valid: true,
        details: "Signature matches ✓".to_string(),
    };

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert!(json.contains("github"));
    assert!(json.contains("sha256=abc123"));
    assert!(json.contains("true"));

    let deserialized: SignatureResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.valid, true);
    assert_eq!(deserialized.provider, "github");
}

#[test]
fn test_output_format_variants() {
    assert_eq!(format!("{:?}", OutputFormat::Text), "Text");
    assert_eq!(format!("{:?}", OutputFormat::Json), "Json");
    assert_eq!(format!("{:?}", OutputFormat::Pretty), "Pretty");
}

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert_eq!(config.response_status, 200);
    assert_eq!(config.response_body, "OK");
    assert!(config.store_captured);
    assert!(config.forward_headers);
    assert_eq!(config.max_body_size, 1024 * 1024);
}

#[test]
fn test_app_state_creation() {
    let config = ServerConfig::default();
    let state = AppState::new(config.clone());
    assert_eq!(state.config.host, "127.0.0.1");
    assert_eq!(state.store.count_sync(), 0);
}

// Helper function
fn create_test_webhook(method: &str, path: &str, body: &str) -> Webhook {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());

    Webhook::new(
        method.to_string(),
        path.to_string(),
        "".to_string(),
        headers,
        serde_json::Value::String(body.to_string()),
        Some(body.to_string()),
        "application/json".to_string(),
        "127.0.0.1:12345".to_string(),
    )
}