# WebTrap 🕸️

**Webhook Testing & Debugging CLI** — capture, inspect, replay, forward, and validate webhooks directly from your terminal.

[![CI](https://github.com/EdgarOrtegaRamirez/webtrap/actions/workflows/ci.yml/badge.svg)](https://github.com/EdgarOrtegaRamirez/webtrap/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## Features

- **📡 Listen** — Start a local HTTP server to capture webhooks in real time
- **🔍 Inspect** — View captured webhooks with full headers, body, and metadata
- **📋 List** — Filter and search captured webhooks by method, path, or tags
- **🔄 Replay** — Replay captured webhooks to any target URL
- **✅ Validate** — Verify webhook signatures (GitHub, GitLab, Stripe, generic HMAC)
- **📤 Export** — Export webhooks to JSON files
- **📥 Import** — Import webhooks from a JSON file (replace or merge mode)
- **📊 Stats** — Show statistics about captured webhooks (methods, paths, timing, body sizes, tags)
- **🏷️ Tag** — Organize webhooks with custom tags
- **🔀 Forward** — Automatically forward incoming webhooks to another endpoint

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/EdgarOrtegaRamirez/webtrap.git
cd webtrap

# Build the release binary
cargo build --release

# Optional: copy to PATH
cp target/release/webtrap ~/.local/bin/
```

### Using Cargo Install

```bash
cargo install --git https://github.com/EdgarOrtegaRamirez/webtrap.git
```

## Quick Start

### 1. Capture Webhooks

Start a webhook capture server on port 8080:

```bash
webtrap listen
```

Or with custom options:

```bash
webtrap listen --port 9000 --host 0.0.0.0 --response-status 201 --response-body '{"status":"ok"}'
```

### 2. Send a test webhook

```bash
curl -X POST http://localhost:8080/my-webhook \
  -H "Content-Type: application/json" \
  -d '{"event":"push","ref":"refs/heads/main"}'
```

### 3. List captured webhooks

```bash
webtrap list
```

### 4. Inspect a specific webhook

```bash
webtrap inspect <webhook-id>
```

### 5. Replay a webhook

```bash
webtrap replay <webhook-id> --target https://example.com/webhook
```

### 6. Validate a webhook signature

```bash
webtrap validate <webhook-id> --secret my-shared-secret --provider github
```

## CLI Reference

### `webtrap listen`

Start a webhook capture server.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--port` | `-p` | `8080` | Port to listen on |
| `--host` | `-h` | `127.0.0.1` | Host to bind to |
| `--response-status` | `-s` | `200` | HTTP status code to return |
| `--response-body` | `-b` | `OK` | Response body to return |
| `--forward-url` | `-f` | — | Forward captured webhooks to this URL |
| `--no-store` | — | — | Don't store captured webhooks |

### `webtrap list`

List captured webhooks with optional filters.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--limit` | `-l` | `20` | Number of webhooks to show |
| `--method` | `-M` | — | Filter by HTTP method |
| `--path` | `-p` | — | Filter by path (contains match) |
| `--tag` | `-t` | — | Filter by tag |
| `--format` | `-f` | `text` | Output format: `text`, `json`, `pretty` |

### `webtrap inspect`

Show detailed information about a webhook.

| Argument | Description |
|----------|-------------|
| `id` | Webhook ID |
| `--format` / `-f` | Output format: `pretty` (default), `json` |

### `webtrap replay`

Replay a captured webhook.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `id` | — | — | Webhook ID |
| `--target` | `-t` | — | Target URL (defaults to original host + path) |
| `--all` | `-a` | — | Replay all recent webhooks |
| `--count` | `-n` | `10` | Number of webhooks to replay (with `--all`) |

### `webtrap validate`

Validate a webhook signature.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `id` | — | — | Webhook ID |
| `--secret` | `-s` | — | Shared secret for validation |
| `--provider` | `-p` | `github` | Provider: `github`, `gitlab`, `stripe`, `generic` |

### `webtrap export`

Export captured webhooks to a file.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--output` | `-o` | `webtrap_export_<timestamp>.json` | Output file path |
| `--format` | `-f` | `json` | Output format |

### `webtrap clear`

Clear all captured webhooks.

| Flag | Short | Description |
|------|-------|-------------|
| `--yes` | `-y` | Skip confirmation prompt |

### `webtrap tag`

Tag a webhook with custom tags.

| Flag | Short | Description |
|------|-------|-------------|
| `id` | — | Webhook ID |
| `--tags` | `-t` | Comma-separated tags |

### `webtrap stats`

Show statistics about captured webhooks.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--format` | `-f` | `text` | Output format: `text`, `json` |

Displays a visual dashboard with:
- Total webhook count
- Time range (earliest, latest, span)
- Body size statistics (min, max, avg, total)
- Method breakdown with ASCII bar charts
- Content type distribution
- Top 10 most frequent paths
- Top 5 source addresses
- Tag distribution

### `webtrap import`

Import webhooks from a JSON file.

| Flag | Short | Default | Description |
|------|-------|---------|-------------|
| `--input` | `-i` | — | Input file path (JSON array of webhooks) |
| `--merge` | — | `false` | Merge with existing webhooks (default: replace all) |

```bash
# Replace all existing webhooks with imported ones
webtrap import --input exported_webhooks.json

# Merge imported webhooks with existing ones
webtrap import --input exported_webhooks.json --merge
```

## Supported Signature Providers

| Provider | Header | Algorithm |
|----------|--------|-----------|
| **GitHub** | `x-hub-signature-256` | HMAC-SHA256 (`sha256=...`) |
| **GitLab** | `x-gitlab-token` | Token comparison |
| **Stripe** | `stripe-signature` | HMAC-SHA256 |
| **Generic** | `x-webhook-signature`, `x-signature` | HMAC-SHA256 with `sha256=...` prefix |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      WebTrap CLI                        │
│                                                         │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌─────────┐  │
│  │ Listen  │  │  List/   │  │  Replay  │  │Validate │  │
│  │ Server  │  │ Inspect  │  │  Engine  │  │         │  │
│  └────┬────┘  └────┬─────┘  └────┬─────┘  └────┬────┘  │
│       │            │             │             │        │
│  ┌────┴────────────┴─────────────┴─────────────┴────┐  │
│  │              WebhookStore (in-memory)             │  │
│  │         + File Persistence (~/.local/share/)      │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

- **Listen Server**: Built with Axum, captures all HTTP methods on any path
- **WebhookStore**: Thread-safe in-memory store with file persistence
- **Replay Engine**: Rebuilds and sends captured requests with original headers
- **Signature Validation**: HMAC-SHA256, token comparison, constant-time comparison

## Security

- No hardcoded secrets — all secrets passed via CLI flags or environment variables
- Constant-time signature comparison to prevent timing attacks
- Input validation on all CLI arguments and API responses
- Connection-specific headers (Host, Content-Length, Transfer-Encoding) filtered during replay
- Graceful shutdown with Ctrl+C and SIGTERM handling

## Development

```bash
# Run tests
cargo test

# Run with verbose logging
RUST_LOG=debug cargo run -- listen

# Build release
cargo build --release
```

## License

MIT — see [LICENSE](LICENSE)