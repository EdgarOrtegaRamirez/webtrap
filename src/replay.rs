use crate::storage::WebhookStore;

/// Replay a webhook by ID
pub async fn replay_webhook(
    store: &WebhookStore,
    id: &str,
    target_url: Option<&str>,
) -> Result<(), String> {
    let webhook = store.get(id).await
        .ok_or_else(|| format!("Webhook with ID '{}' not found", id))?;

    let url: String = if let Some(t) = target_url {
        t.to_string()
    } else {
        // If the webhook has a host header, use it
        webhook.headers.get("host")
            .map(|h| format!("http://{}{}", h, webhook.path))
            .unwrap_or_else(|| {
                let prefix = if webhook.path.starts_with('/') { "" } else { "/" };
                format!("http://localhost{}{}", prefix, webhook.path)
            })
    };

    let method = reqwest::Method::from_bytes(webhook.method.as_bytes())
        .map_err(|e| format!("Invalid HTTP method '{}': {}", webhook.method, e))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut req = client.request(method.clone(), &url);

    // Add headers (skip connection-specific ones)
    for (key, value) in &webhook.headers {
        let lower = key.to_lowercase();
        if lower == "host" || lower == "content-length" || lower == "transfer-encoding" {
            continue;
        }
        if let Ok(name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(value) {
                req = req.header(name, val);
            }
        }
    }

    // Add body if present
    if let Some(ref raw_body) = webhook.raw_body {
        if !raw_body.is_empty() {
            req = req.body(raw_body.clone());
        }
    }

    println!("Replaying webhook {} to {} {} ...", webhook.id, method, url);

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            let body_preview = if body.len() > 500 {
                format!("{}... ({} bytes)", &body[..500], body.len())
            } else {
                body
            };
            println!("Response: {} ({})", status, if status < 400 { "OK" } else { "Error" });
            println!("Body: {}", body_preview);
            Ok(())
        }
        Err(e) => {
            Err(format!("Failed to replay webhook: {}", e))
        }
    }
}

/// Replay multiple webhooks in sequence
pub async fn replay_all(
    store: &WebhookStore,
    target_url: Option<&str>,
    count: Option<usize>,
) -> Result<(), String> {
    let webhooks = store.recent(count.unwrap_or(10)).await;
    if webhooks.is_empty() {
        println!("No webhooks to replay.");
        return Ok(());
    }

    println!("Replaying {} webhooks...", webhooks.len());
    for (i, wh) in webhooks.iter().enumerate() {
        let id_str = wh.id.to_string();
        match replay_webhook(store, &id_str, target_url).await {
            Ok(()) => println!("[{}/{}] ✓ Replayed {}", i + 1, webhooks.len(), id_str),
            Err(e) => println!("[{}/{}] ✗ {}: {}", i + 1, webhooks.len(), id_str, e),
        }
    }
    Ok(())
}