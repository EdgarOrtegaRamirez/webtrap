/// Validate webhook signatures
use crate::storage::WebhookStore;
use crate::types::SignatureResult;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

type HmacSha256 = Hmac<Sha256>;

/// Supported webhook providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    GitLab,
    Stripe,
    Generic,
}

impl Provider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "github" => Some(Self::GitHub),
            "gitlab" => Some(Self::GitLab),
            "stripe" => Some(Self::Stripe),
            "generic" => Some(Self::Generic),
            _ => None,
        }
    }

    fn signature_headers(&self) -> &[&str] {
        match self {
            Self::GitHub => &["x-hub-signature-256", "x-hub-signature"],
            Self::GitLab => &["x-gitlab-token"],
            Self::Stripe => &["stripe-signature"],
            Self::Generic => &["x-webhook-signature", "x-signature"],
        }
    }
}

/// Validate a webhook signature
pub async fn validate_signature(
    store: &WebhookStore,
    id: &str,
    secret: &str,
    provider: Option<&str>,
) -> Result<SignatureResult, String> {
    let webhook = store.get(id).await
        .ok_or_else(|| format!("Webhook with ID '{}' not found", id))?;

    let provider_name = provider.unwrap_or("github");
    let provider = Provider::from_str(provider_name)
        .ok_or_else(|| format!("Unknown provider: {}. Supported: github, gitlab, stripe, generic", provider_name))?;

    let headers = &webhook.headers;

    // Find the signature header
    let sig_header = provider.signature_headers().iter()
        .find(|h| headers.contains_key(**h))
        .copied()
        .ok_or_else(|| {
            let expected = provider.signature_headers().join(", ");
            format!("No signature header found. Expected one of: {}", expected)
        })?;

    let sig_value = headers.get(sig_header).cloned().unwrap();
    let raw_body = webhook.raw_body.clone().unwrap_or_default();

    // Compute expected signature
    let computed = match provider {
        Provider::GitHub => {
            // GitHub sends sha256=hexdigest
            let body = if sig_value.starts_with("sha256=") {
                let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                    .map_err(|e| format!("HMAC initialization error: {}", e))?;
                mac.update(raw_body.as_bytes());
                let result = mac.finalize();
                format!("sha256={}", hex::encode(result.into_bytes()))
            } else if sig_value.starts_with("sha1=") {
                // SHA1 - not supported
                return Err("SHA1 signatures not supported. Use SHA256.".to_string());
            } else {
                return Err(format!("Unknown signature format: {}", sig_value));
            };
            body
        }
        Provider::GitLab => {
            // GitLab uses a shared token in the header
            if sig_value == secret {
                sig_value.clone()
            } else {
                format!("invalid (expected: {})", secret)
            }
        }
        Provider::Stripe => {
            // Stripe uses timestamped signatures
            // For simplicity, we compute HMAC-SHA256 of the body
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| format!("HMAC initialization error: {}", e))?;
            mac.update(raw_body.as_bytes());
            let result = mac.finalize();
            hex::encode(result.into_bytes())
        }
        Provider::Generic => {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| format!("HMAC initialization error: {}", e))?;
            mac.update(raw_body.as_bytes());
            let result = mac.finalize();
            format!("sha256={}", hex::encode(result.into_bytes()))
        }
    };

    let valid = if provider == Provider::GitLab {
        sig_value == secret
    } else {
        // Constant-time comparison for HMAC
        use subtle::ConstantTimeEq;
        let computed_bytes = computed.as_bytes();
        let sig_bytes = sig_value.as_bytes();
        // Truncate to shorter length for comparison
        let min_len = std::cmp::min(computed_bytes.len(), sig_bytes.len());
        let result = computed_bytes[..min_len].ct_eq(&sig_bytes[..min_len]);
        result.into()
    };

    Ok(SignatureResult {
        webhook_id: webhook.id,
        provider: provider_name.to_string(),
        signature_header: sig_header.to_string(),
        signature_value: sig_value,
        computed_signature: computed,
        valid,
        details: if valid {
            "Signature matches ✓".to_string()
        } else {
            "Signature does NOT match ✗".to_string()
        },
    })
}