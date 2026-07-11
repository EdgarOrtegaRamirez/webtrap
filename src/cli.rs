use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// WebTrap — Webhook Testing & Debugging CLI
///
/// Capture, inspect, replay, forward, and validate webhooks from your terminal.
#[derive(Parser)]
#[command(name = "webtrap", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a webhook capture server
    Listen {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Host to bind to
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// Response status code to return
        #[arg(short = 's', long, default_value = "200")]
        response_status: u16,

        /// Response body to return
        #[arg(short = 'b', long, default_value = "OK")]
        response_body: String,

        /// Forward captured webhooks to this URL
        #[arg(short = 'f', long)]
        forward_url: Option<String>,

        /// Don't store captured webhooks in memory
        #[arg(long)]
        no_store: bool,
    },

    /// List captured webhooks
    List {
        /// Number of webhooks to show
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Filter by HTTP method
        #[arg(short = 'M', long)]
        method: Option<String>,

        /// Filter by path (contains match)
        #[arg(short = 'p', long)]
        path: Option<String>,

        /// Filter by tag
        #[arg(short = 't', long)]
        tag: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: super::types::OutputFormat,
    },

    /// Show detailed information about a webhook
    Inspect {
        /// Webhook ID
        id: String,

        /// Output format
        #[arg(short, long, value_enum, default_value = "pretty")]
        format: super::types::OutputFormat,
    },

    /// Replay a captured webhook
    Replay {
        /// Webhook ID
        id: String,

        /// Target URL (defaults to original host + path)
        #[arg(short, long)]
        target: Option<String>,

        /// Replay all recent webhooks
        #[arg(short, long)]
        all: bool,

        /// Number of webhooks to replay (with --all)
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
    },

    /// Validate webhook signature
    Validate {
        /// Webhook ID
        id: String,

        /// Shared secret for validation
        #[arg(short, long)]
        secret: String,

        /// Provider (github, gitlab, stripe, generic)
        #[arg(short, long, default_value = "github")]
        provider: String,
    },

    /// Export captured webhooks to a file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (json only)
        #[arg(short, long, value_enum, default_value = "json")]
        format: super::types::OutputFormat,
    },

    /// Clear all captured webhooks
    Clear {
        /// Confirm clearing
        #[arg(short, long)]
        yes: bool,
    },

    /// Tag a webhook
    Tag {
        /// Webhook ID
        id: String,

        /// Tags to apply (comma-separated)
        #[arg(short, long)]
        tags: String,
    },

    /// Show statistics about captured webhooks
    Stats {
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: super::types::OutputFormat,
    },

    /// Import webhooks from a JSON file
    Import {
        /// Input file path (JSON array of webhooks)
        #[arg(short, long)]
        input: std::path::PathBuf,

        /// Merge with existing webhooks (default: replace)
        #[arg(long)]
        merge: bool,
    },
}
