use crate::storage::WebhookStore;
use crate::types::OutputFormat;
use colored::*;
use std::collections::HashMap;

/// Aggregated statistics about captured webhooks
#[derive(Debug, serde::Serialize)]
pub struct WebhookStats {
    pub total: usize,
    pub by_method: HashMap<String, usize>,
    pub by_path: HashMap<String, usize>,
    pub by_content_type: HashMap<String, usize>,
    pub by_source: HashMap<String, usize>,
    pub by_status_tag: HashMap<String, usize>,
    pub time_range: Option<TimeRange>,
    pub body_sizes: BodySizeStats,
    pub top_paths: Vec<(String, usize)>,
    pub top_sources: Vec<(String, usize)>,
}

#[derive(Debug, serde::Serialize)]
pub struct TimeRange {
    pub earliest: chrono::DateTime<chrono::Utc>,
    pub latest: chrono::DateTime<chrono::Utc>,
    pub span_seconds: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct BodySizeStats {
    pub min: usize,
    pub max: usize,
    pub avg: f64,
    pub total_bytes: usize,
}

/// Compute statistics from the webhook store
pub async fn compute_stats(store: &WebhookStore) -> WebhookStats {
    let webhooks = store.all().await;
    let total = webhooks.len();

    let mut by_method: HashMap<String, usize> = HashMap::new();
    let mut by_path: HashMap<String, usize> = HashMap::new();
    let mut by_content_type: HashMap<String, usize> = HashMap::new();
    let mut by_source: HashMap<String, usize> = HashMap::new();
    let mut by_status_tag: HashMap<String, usize> = HashMap::new();

    let mut earliest: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;

    let mut min_body = usize::MAX;
    let mut max_body = 0usize;
    let mut total_bytes = 0usize;

    for wh in &webhooks {
        *by_method.entry(wh.method.clone()).or_default() += 1;
        *by_path.entry(wh.path.clone()).or_default() += 1;
        *by_content_type.entry(wh.content_type.clone()).or_default() += 1;
        *by_source.entry(wh.source_addr.clone()).or_default() += 1;

        // Tag-based grouping
        if wh.tags.is_empty() {
            *by_status_tag.entry("untagged".to_string()).or_default() += 1;
        } else {
            for tag in &wh.tags {
                *by_status_tag.entry(tag.clone()).or_default() += 1;
            }
        }

        // Time range
        if earliest.is_none() || wh.received_at < earliest.unwrap() {
            earliest = Some(wh.received_at);
        }
        if latest.is_none() || wh.received_at > latest.unwrap() {
            latest = Some(wh.received_at);
        }

        // Body sizes
        let body_len = wh
            .raw_body
            .as_ref()
            .map(|b| b.len())
            .unwrap_or_else(|| wh.body.to_string().len());
        min_body = min_body.min(body_len);
        max_body = max_body.max(body_len);
        total_bytes += body_len;
    }

    let time_range = match (earliest, latest) {
        (Some(e), Some(l)) => Some(TimeRange {
            earliest: e,
            latest: l,
            span_seconds: (l - e).num_seconds() as f64,
        }),
        _ => None,
    };

    let body_sizes = if total == 0 {
        BodySizeStats {
            min: 0,
            max: 0,
            avg: 0.0,
            total_bytes: 0,
        }
    } else {
        BodySizeStats {
            min: if min_body == usize::MAX { 0 } else { min_body },
            max: max_body,
            avg: total_bytes as f64 / total as f64,
            total_bytes,
        }
    };

    // Top paths and sources (sorted by count descending)
    let mut top_paths: Vec<(String, usize)> = by_path.into_iter().collect();
    top_paths.sort_by_key(|b| std::cmp::Reverse(b.1));
    top_paths.truncate(10);

    let mut top_sources: Vec<(String, usize)> = by_source.into_iter().collect();
    top_sources.sort_by_key(|b| std::cmp::Reverse(b.1));
    top_sources.truncate(10);

    WebhookStats {
        total,
        by_method,
        by_path: top_paths.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        by_content_type,
        by_source: top_sources.iter().map(|(k, v)| (k.clone(), *v)).collect(),
        by_status_tag,
        time_range,
        body_sizes,
        top_paths,
        top_sources,
    }
}

/// Display statistics in the terminal
pub async fn display_stats(store: &WebhookStore, format: OutputFormat) -> Result<(), String> {
    let stats = compute_stats(store).await;

    if stats.total == 0 {
        println!("No webhooks captured yet.");
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&stats)
                .map_err(|e| format!("JSON serialization error: {}", e))?;
            println!("{}", json);
        }
        _ => {
            print_stats_text(&stats);
        }
    }

    Ok(())
}

fn print_stats_text(stats: &WebhookStats) {
    println!(
        "{}",
        "╔═ WebTrap Statistics ════════════════════════╗".cyan()
    );
    println!(
        "  {} {}",
        "Total webhooks:".bold(),
        stats.total.to_string().yellow()
    );

    // Time range
    if let Some(ref tr) = stats.time_range {
        println!("  {}", "Time Range:".bold());
        println!(
            "    Earliest: {}",
            tr.earliest.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!(
            "    Latest:   {}",
            tr.latest.format("%Y-%m-%d %H:%M:%S UTC")
        );
        let span = tr.span_seconds;
        let span_str = if span < 60.0 {
            format!("{:.1}s", span)
        } else if span < 3600.0 {
            format!("{:.1}m", span / 60.0)
        } else if span < 86400.0 {
            format!("{:.1}h", span / 3600.0)
        } else {
            format!("{:.1}d", span / 86400.0)
        };
        println!("    Span:     {}", span_str.yellow());
    }

    // Body sizes
    println!("  {}", "Body Sizes:".bold());
    println!(
        "    Min: {} | Max: {} | Avg: {:.1} bytes",
        format_bytes(stats.body_sizes.min),
        format_bytes(stats.body_sizes.max),
        stats.body_sizes.avg
    );
    println!("    Total: {}", format_bytes(stats.body_sizes.total_bytes));

    // Methods breakdown
    println!("  {}", "By Method:".bold());
    let mut methods: Vec<_> = stats.by_method.iter().collect();
    methods.sort_by(|a, b| b.1.cmp(a.1));
    for (method, count) in &methods {
        let pct = (**count as f64 / stats.total as f64) * 100.0;
        println!(
            "    {} {:>5} ({:>5.1}%) {}",
            method.green(),
            count,
            pct,
            bar(**count, stats.total)
        );
    }

    // Content types
    if !stats.by_content_type.is_empty() {
        println!("  {}", "By Content Type:".bold());
        let mut cts: Vec<_> = stats.by_content_type.iter().collect();
        cts.sort_by(|a, b| b.1.cmp(a.1));
        for (ct, count) in cts.iter().take(5) {
            println!("    {} ({})", ct, count);
        }
    }

    // Top paths
    if !stats.top_paths.is_empty() {
        println!("  {}", "Top Paths:".bold());
        for (path, count) in stats.top_paths.iter().take(10) {
            println!("    {:>3}x {}", count, path.yellow());
        }
    }

    // Top sources
    if !stats.top_sources.is_empty() {
        println!("  {}", "Top Sources:".bold());
        for (source, count) in stats.top_sources.iter().take(5) {
            println!("    {:>3}x {}", count, source);
        }
    }

    // Tags
    if !stats.by_status_tag.is_empty() {
        println!("  {}", "Tags:".bold());
        let mut tags: Vec<_> = stats.by_status_tag.iter().collect();
        tags.sort_by(|a, b| b.1.cmp(a.1));
        for (tag, count) in tags.iter().take(10) {
            println!("    {} ({})", tag, count);
        }
    }

    println!(
        "{}",
        "╚════════════════════════════════════════════╝".cyan()
    );
}

fn bar(count: usize, total: usize) -> String {
    let width = 20;
    if total == 0 {
        return String::new();
    }
    let filled = (count as f64 / total as f64 * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!(
        "{}{}",
        "█".repeat(filled).green(),
        "░".repeat(empty).dimmed()
    )
}

fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Webhook;

    fn make_webhook(method: &str, path: &str, body: &str) -> Webhook {
        Webhook::new(
            method.to_string(),
            path.to_string(),
            String::new(),
            std::collections::HashMap::new(),
            serde_json::Value::String(body.to_string()),
            Some(body.to_string()),
            "application/json".to_string(),
            "127.0.0.1:12345".to_string(),
        )
    }

    #[tokio::test]
    async fn test_stats_empty() {
        let store = WebhookStore::new();
        let stats = compute_stats(&store).await;
        assert_eq!(stats.total, 0);
        assert!(stats.time_range.is_none());
        assert_eq!(stats.body_sizes.total_bytes, 0);
    }

    #[tokio::test]
    async fn test_stats_with_webhooks() {
        let store = WebhookStore::new();
        store
            .add(make_webhook("POST", "/webhook", r#"{"event":"push"}"#))
            .await;
        store
            .add(make_webhook("GET", "/health", r#"{"status":"ok"}"#))
            .await;
        store
            .add(make_webhook("POST", "/webhook", r#"{"event":"pull"}"#))
            .await;

        let stats = compute_stats(&store).await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.by_method.get("POST"), Some(&2));
        assert_eq!(stats.by_method.get("GET"), Some(&1));
        assert!(stats.time_range.is_some());
        assert!(stats.body_sizes.total_bytes > 0);
        assert!(stats.body_sizes.avg > 0.0);
    }

    #[tokio::test]
    async fn test_stats_top_paths() {
        let store = WebhookStore::new();
        for _ in 0..5 {
            store.add(make_webhook("POST", "/webhook", "x")).await;
        }
        for _ in 0..2 {
            store.add(make_webhook("GET", "/health", "y")).await;
        }
        store.add(make_webhook("POST", "/other", "z")).await;

        let stats = compute_stats(&store).await;
        assert_eq!(stats.top_paths.len(), 3);
        assert_eq!(stats.top_paths[0], ("/webhook".to_string(), 5));
        assert_eq!(stats.top_paths[1], ("/health".to_string(), 2));
        assert_eq!(stats.top_paths[2], ("/other".to_string(), 1));
    }

    #[tokio::test]
    async fn test_stats_body_sizes() {
        let store = WebhookStore::new();
        store.add(make_webhook("POST", "/a", "short")).await;
        store
            .add(make_webhook("POST", "/b", "a much longer body string here"))
            .await;

        let stats = compute_stats(&store).await;
        assert!(stats.body_sizes.min < stats.body_sizes.max);
        assert!(stats.body_sizes.max > 0);
        assert_eq!(
            stats.body_sizes.total_bytes,
            "short".len() + "a much longer body string here".len()
        );
    }

    #[tokio::test]
    async fn test_display_stats_empty() {
        let store = WebhookStore::new();
        let result = display_stats(&store, OutputFormat::Text).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_display_stats_json() {
        let store = WebhookStore::new();
        store
            .add(make_webhook("POST", "/webhook", r#"{"x":1}"#))
            .await;
        let result = display_stats(&store, OutputFormat::Json).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stats_with_tags() {
        let store = WebhookStore::new();
        let mut wh = make_webhook("POST", "/webhook", "data");
        wh.tags = vec!["production".to_string(), "critical".to_string()];
        store.add(wh).await;
        store.add(make_webhook("GET", "/health", "ok")).await;

        let stats = compute_stats(&store).await;
        assert_eq!(stats.by_status_tag.get("production"), Some(&1));
        assert_eq!(stats.by_status_tag.get("critical"), Some(&1));
        assert_eq!(stats.by_status_tag.get("untagged"), Some(&1));
    }
}
