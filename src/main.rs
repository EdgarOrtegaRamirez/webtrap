use clap::Parser;
use std::net::SocketAddr;
use tracing::{info, warn};

use webtrap::cli::{self, Commands};
use webtrap::inspect;
use webtrap::replay;
use webtrap::server;
use webtrap::stats;
use webtrap::storage;
use webtrap::types;
use webtrap::validate;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = cli::Cli::parse();

    // Use a shared state file for persistence across commands
    let state_file = dirs_state_file();

    match cli.command {
        Commands::Listen {
            port,
            host,
            response_status,
            response_body,
            forward_url,
            no_store,
        } => {
            cmd_listen(
                port,
                host,
                response_status,
                response_body,
                forward_url,
                no_store,
            )
            .await
        }
        Commands::List {
            limit,
            method,
            path,
            tag,
            format,
        } => cmd_list(limit, method, path, tag, format, &state_file).await,
        Commands::Inspect { id, format } => cmd_inspect(id, format, &state_file).await,
        Commands::Replay {
            id,
            target,
            all,
            count,
        } => cmd_replay(id, target, all, count, &state_file).await,
        Commands::Validate {
            id,
            secret,
            provider,
        } => cmd_validate(id, &secret, &provider, &state_file).await,
        Commands::Export { output, format } => cmd_export(output, format, &state_file).await,
        Commands::Clear { yes } => cmd_clear(yes, &state_file).await,
        Commands::Tag { id, tags } => cmd_tag(id, &tags, &state_file).await,
        Commands::Stats { format } => cmd_stats(format, &state_file).await,
        Commands::Import { input, merge } => cmd_import(input, merge, &state_file).await,
    }
}

fn dirs_state_file() -> std::path::PathBuf {
    let data_dir = dirs_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    data_dir.join("webtrap_state.json")
}

fn dirs_data_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("webtrap")
}

async fn load_webhooks(path: &std::path::Path) -> storage::WebhookStore {
    let store = storage::WebhookStore::new();
    if path.exists() {
        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                if let Ok(webhooks) = serde_json::from_str::<Vec<types::Webhook>>(&content) {
                    for wh in webhooks {
                        store.add(wh).await;
                    }
                    let count = store.count().await;
                    info!("Loaded {} webhooks from state file", count);
                }
            }
            Err(e) => warn!("Failed to read state file: {}", e),
        }
    }
    store
}

async fn save_webhooks(store: &storage::WebhookStore, path: &std::path::Path) {
    let json = store
        .export_json()
        .await
        .unwrap_or_else(|_| "[]".to_string());
    tokio::fs::write(path, json).await.ok();
}

/// Listen command: start a webhook capture server
async fn cmd_listen(
    port: u16,
    host: String,
    response_status: u16,
    response_body: String,
    forward_url: Option<String>,
    no_store: bool,
) {
    let config = types::ServerConfig {
        host: host.clone(),
        port,
        response_status,
        response_body,
        forward_url,
        store_captured: !no_store,
        ..Default::default()
    };

    let state = storage::AppState::new(config);
    let app = server::build_server(state.clone());

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid host/port combination");

    info!("WebTrap listening on http://{}", addr);
    info!("Send webhooks to http://{}:{}/<path>", host, port);
    if let Some(ref forward_url) = state.config.forward_url {
        info!("Forwarding webhooks to: {}", forward_url);
    }
    println!("🚀 WebTrap server running at http://{}", addr);
    println!("   Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n👋 WebTrap server stopped.");
}

// List command: show captured webhooks
async fn cmd_list(
    limit: usize,
    method: Option<String>,
    path: Option<String>,
    tag: Option<String>,
    format: types::OutputFormat,
    state_file: &std::path::Path,
) {
    let store = load_webhooks(state_file).await;
    let filter = types::WebhookFilter {
        method,
        path,
        tag,
        limit: Some(limit),
        ..Default::default()
    };
    if let Err(e) = inspect::display_webhooks(&store, &filter, format).await {
        eprintln!("Error: {}", e);
    }
}

// Inspect command
async fn cmd_inspect(id: String, format: types::OutputFormat, state_file: &std::path::Path) {
    let store = load_webhooks(state_file).await;
    if let Err(e) = inspect::show_webhook_detail(&store, &id, format).await {
        eprintln!("Error: {}", e);
    }
}

// Replay command
async fn cmd_replay(
    id: String,
    target: Option<String>,
    all: bool,
    count: usize,
    state_file: &std::path::Path,
) {
    let store = load_webhooks(state_file).await;
    let target_url = target.as_deref();
    if all {
        if let Err(e) = replay::replay_all(&store, target_url, Some(count)).await {
            eprintln!("Error: {}", e);
        }
    } else if let Err(e) = replay::replay_webhook(&store, &id, target_url).await {
        eprintln!("Error: {}", e);
    }
}

// Validate command
async fn cmd_validate(id: String, secret: &str, provider: &str, state_file: &std::path::Path) {
    let store = load_webhooks(state_file).await;
    match validate::validate_signature(&store, &id, secret, Some(provider)).await {
        Ok(result) => {
            println!("Webhook ID:   {}", result.webhook_id);
            println!("Provider:     {}", result.provider);
            println!("Header:       {}", result.signature_header);
            println!("Value:        {}", result.signature_value);
            println!("Computed:     {}", result.computed_signature);
            println!(
                "Status:       {}",
                if result.valid {
                    "✅ VALID"
                } else {
                    "❌ INVALID"
                }
            );
            println!("Details:      {}", result.details);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

// Export command
async fn cmd_export(
    output: Option<std::path::PathBuf>,
    _format: types::OutputFormat,
    state_file: &std::path::Path,
) {
    let store = load_webhooks(state_file).await;
    let count = store.count().await;
    let output_path = output.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        std::path::PathBuf::from(format!("webtrap_export_{}.json", timestamp))
    });
    let json = store
        .export_json()
        .await
        .unwrap_or_else(|_| "[]".to_string());
    tokio::fs::write(&output_path, json).await.ok();
    println!("Exported {} webhooks to {}", count, output_path.display());
}

// Clear command
async fn cmd_clear(yes: bool, state_file: &std::path::Path) {
    if !yes {
        print!("Are you sure you want to clear all webhooks? [y/N] ");
        use std::io::Write;
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return;
        }
    }
    let store = load_webhooks(state_file).await;
    let count = store.count().await;
    store.clear().await;
    save_webhooks(&store, state_file).await;
    println!("Cleared {} webhooks.", count);
}

// Tag command
async fn cmd_tag(id: String, tags: &str, state_file: &std::path::Path) {
    let store = load_webhooks(state_file).await;
    let tag_list: Vec<String> = tags.split(',').map(|s| s.trim().to_string()).collect();
    if store.tag(&id, tag_list.clone()).await {
        save_webhooks(&store, state_file).await;
        println!("Tagged webhook {} with: {}", id, tag_list.join(", "));
    } else {
        eprintln!("Webhook with ID '{}' not found", id);
    }
}

// Stats command: show statistics about captured webhooks
async fn cmd_stats(format: types::OutputFormat, state_file: &std::path::Path) {
    let store = load_webhooks(state_file).await;
    if let Err(e) = stats::display_stats(&store, format).await {
        eprintln!("Error: {}", e);
    }
}

// Import command: import webhooks from a JSON file
async fn cmd_import(input: std::path::PathBuf, merge: bool, state_file: &std::path::Path) {
    // Validate the file path (prevent path traversal in CI contexts)
    let canonical = match input.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "Error: Cannot access input file '{}': {}",
                input.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let content = match tokio::fs::read_to_string(&canonical).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Error: Failed to read file '{}': {}",
                canonical.display(),
                e
            );
            std::process::exit(1);
        }
    };

    let imported: Vec<types::Webhook> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: Failed to parse JSON: {}", e);
            std::process::exit(1);
        }
    };

    let import_count = imported.len();

    let store = if merge {
        load_webhooks(state_file).await
    } else {
        storage::WebhookStore::new()
    };

    for wh in imported {
        store.add(wh).await;
    }

    save_webhooks(&store, state_file).await;
    let total = store.count().await;

    if merge {
        println!(
            "Imported {} webhooks (merged). Total now: {}",
            import_count, total
        );
    } else {
        println!(
            "Imported {} webhooks (replaced existing). Total: {}",
            import_count, total
        );
    }
}
