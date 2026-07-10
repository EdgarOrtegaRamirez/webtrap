# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in WebTrap, please report it by opening an issue at https://github.com/EdgarOrtegaRamirez/webtrap/issues.

Please do not report security vulnerabilities through public GitHub issues if they could be exploited in production.

## Security Considerations

### Signature Validation

WebTrap uses constant-time comparison (`subtle::ConstantTimeEq`) for HMAC signature validation to prevent timing side-channel attacks. Always use the `--provider` flag that matches the webhook source to ensure correct signature verification.

### Webhook Forwarding

When using the `--forward-url` flag, webhooks are forwarded to the specified URL. Ensure the target URL uses HTTPS in production environments to protect webhook payloads in transit.

### Local Server

The webhook capture server binds to `127.0.0.1` by default for security. When binding to `0.0.0.0`, ensure appropriate network-level protections (firewall, authentication) are in place.

### Secrets

- Never hardcode secrets in scripts or configuration files
- Pass secrets via CLI flags or environment variables
- The `--secret` flag for signature validation is visible in process listings — use with caution in shared environments

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | ✅ Yes             |

## Best Practices

1. Always validate webhook signatures in production
2. Use HTTPS for forwarding URLs
3. Restrict binding to localhost unless necessary
4. Clear captured webhooks after processing sensitive data
5. Use `RUST_LOG=warn` in production to reduce log output