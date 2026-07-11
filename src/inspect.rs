use crate::storage::WebhookStore;
use crate::types::{OutputFormat, Webhook, WebhookFilter};
use colored::*;

/// Display webhooks in the terminal
pub async fn display_webhooks(
    store: &WebhookStore,
    filter: &WebhookFilter,
    format: OutputFormat,
) -> Result<(), String> {
    let webhooks = store.all().await;
    let filtered = apply_filter(&webhooks, filter);

    if filtered.is_empty() {
        println!("No webhooks captured.");
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&filtered)
                .map_err(|e| format!("JSON serialization error: {}", e))?;
            println!("{}", json);
        }
        OutputFormat::Pretty => {
            for (i, wh) in filtered.iter().enumerate() {
                println!("{}", wh.summary());
                if i < filtered.len() - 1 {
                    println!();
                }
            }
        }
        OutputFormat::Text => {
            for (i, wh) in filtered.iter().enumerate() {
                println!(
                    "[{}] {} {} | {} | {} | {}",
                    i + 1,
                    wh.method.cyan().bold(),
                    wh.path.yellow(),
                    wh.received_at.format("%H:%M:%S"),
                    wh.source_addr,
                    format_body_preview(&wh.body, 60),
                );
            }
            println!(
                "\nTotal: {} webhooks (filtered from {})",
                filtered.len(),
                webhooks.len()
            );
        }
    }

    Ok(())
}

/// Show detailed info about a single webhook
pub async fn show_webhook_detail(
    store: &WebhookStore,
    id: &str,
    format: OutputFormat,
) -> Result<(), String> {
    let webhook = store
        .get(id)
        .await
        .ok_or_else(|| format!("Webhook with ID '{}' not found", id))?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&webhook)
                .map_err(|e| format!("JSON serialization error: {}", e))?;
            println!("{}", json);
        }
        _ => {
            println!("{}", webhook.summary());
            if let Some(ref raw) = webhook.raw_body
                && raw.len() > webhook.body.to_string().len() + 50
            {
                println!("\nRaw body ({} bytes):", raw.len());
                println!("{}", raw);
            }
            println!("\nHeaders ({}):", webhook.headers.len());
            let mut sorted_headers: Vec<_> = webhook.headers.iter().collect();
            sorted_headers.sort_by_key(|(k, _)| *k);
            for (key, value) in &sorted_headers {
                println!("  {}: {}", key.cyan(), value);
            }
            if !webhook.query.is_empty() {
                println!("\nQuery string: {}", webhook.query);
            }
        }
    }

    Ok(())
}

fn apply_filter(webhooks: &[Webhook], filter: &WebhookFilter) -> Vec<Webhook> {
    let mut filtered: Vec<Webhook> = webhooks
        .iter()
        .filter(|w| {
            if let Some(ref method) = filter.method
                && !w.method.eq_ignore_ascii_case(method)
            {
                return false;
            }
            if let Some(ref path) = filter.path
                && !w.path.contains(path)
            {
                return false;
            }
            if let Some(ref tag) = filter.tag
                && !w.tags.iter().any(|t| t.contains(tag))
            {
                return false;
            }
            if let Some(ref source) = filter.source
                && !w.source_addr.contains(source)
            {
                return false;
            }
            if let Some(since) = filter.since
                && w.received_at < since
            {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    // Sort by most recent first
    filtered.sort_by_key(|w| std::cmp::Reverse(w.received_at));

    // Apply limit
    if let Some(limit) = filter.limit {
        filtered.truncate(limit);
    }

    filtered
}

fn format_body_preview(body: &serde_json::Value, max_len: usize) -> String {
    let s = match body {
        serde_json::Value::String(s) => s.clone(),
        _ => body.to_string(),
    };
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s
    }
}
