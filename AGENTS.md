# AGENTS.md — AI Agent Guide for WebTrap

## Overview

WebTrap is a Rust CLI tool for webhook testing and debugging. It provides a local HTTP server to capture webhooks, inspect them, replay them, validate signatures, and export them.

## Project Structure

```
webtrap/
├── src/
│   ├── main.rs        # CLI entry point with command handlers
│   ├── lib.rs         # Library exports
│   ├── cli.rs         # CLI argument definitions (clap)
│   ├── types.rs       # Core data types (Webhook, ServerConfig, etc.)
│   ├── storage.rs     # Thread-safe webhook storage
│   ├── server.rs      # Axum HTTP server for webhook capture
│   ├── inspect.rs     # Display/formatting logic
│   ├── replay.rs      # Webhook replay engine
│   └── validate.rs    # Signature validation (GitHub, GitLab, Stripe)
├── tests/
│   └── integration.rs # Integration tests
├── Cargo.toml
└── README.md
```

## Key Architecture

- **WebhookStore**: In-memory `Arc<RwLock<Vec<Webhook>>>` with async and sync methods
- **AppState**: Shared state containing store, config, and stats
- **Server**: Axum router with catch-all route, CORS support, and optional forwarding
- **Signature Validation**: HMAC-SHA256 with constant-time comparison via `subtle` crate

## Dependencies

- `clap` — CLI argument parsing
- `axum` — HTTP server
- `tokio` — Async runtime
- `reqwest` — HTTP client (rustls-tls)
- `serde` / `serde_json` — Serialization
- `hmac` / `sha2` / `hex` — Signature computation
- `subtle` — Constant-time comparison
- `chrono` — Timestamps
- `uuid` — Webhook IDs
- `colored` — Terminal output formatting
- `tower-http` — CORS middleware
- `tracing` — Logging

## Building

```bash
cargo build --release
cargo test
```

## Common Tasks

### Adding a new provider for signature validation

1. Add a variant to `Provider` enum in `validate.rs`
2. Implement the signature header name and algorithm
3. Add validation logic in `validate_signature()`

### Adding a new CLI command

1. Add variant to `Commands` enum in `cli.rs`
2. Add handler function in `main.rs`
3. Add command to `match` block

### Adding a new output format

1. Add variant to `OutputFormat` enum in `types.rs`
2. Add display logic in `inspect.rs`