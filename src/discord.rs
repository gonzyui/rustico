use crate::models::{Component, DiscordWebhookV2, COMPONENTS_V2_FLAG};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use std::time::Duration;
use tracing::{debug, info, warn};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Sends a Discord Components V2 message to a webhook with automatic
/// rate limit handling and exponential backoff retry on transient failures.
///
/// Parses `X-RateLimit-*` response headers to preemptively wait when the
/// bucket is exhausted, and respects `Retry-After` on 429 responses.
/// Retries up to 3 times with 1s → 2s → 4s backoff on network errors
/// and 5xx server errors.
pub async fn send_discord(
    client: &reqwest::Client,
    webhook_url: &str,
    username: &str,
    components: Vec<Component>,
) -> Result<()> {
    let payload = DiscordWebhookV2 {
        username: username.to_string(),
        avatar_url: None,
        flags: COMPONENTS_V2_FLAG,
        components,
    };

    let url = if webhook_url.contains('?') {
        format!("{}&with_components=true", webhook_url)
    } else {
        format!("{}?with_components=true", webhook_url)
    };

    for attempt in 0..MAX_RETRIES {
        let res = match client.post(&url).json(&payload).send().await {
            Ok(r) => r,
            Err(e) if attempt < MAX_RETRIES - 1 => {
                let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
                warn!(
                    "Discord request failed (attempt {}/{}), retrying in {}ms: {}",
                    attempt + 1,
                    MAX_RETRIES,
                    backoff,
                    e
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                continue;
            }
            Err(e) => return Err(e.into()),
        };

        if let Some(remaining) = res.headers().get("x-ratelimit-remaining") {
            if let Ok(val) = remaining.to_str().unwrap_or("").parse::<u32>() {
                if val == 0 {
                    if let Some(reset_after) = res.headers().get("x-ratelimit-reset-after") {
                        if let Ok(secs) = reset_after.to_str().unwrap_or("").parse::<f64>() {
                            let wait = Duration::from_secs_f64(secs);
                            debug!(
                                "Rate limit bucket exhausted, preemptively waiting {:?}",
                                wait
                            );
                            tokio::time::sleep(wait).await;
                        }
                    }
                }
            }
        }

        let status = res.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = res
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(5.0);

            warn!(
                "Rate limited by Discord (attempt {}/{}), waiting {:.1}s",
                attempt + 1,
                MAX_RETRIES,
                retry_after
            );
            tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
            continue;
        }

        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();

            if attempt < MAX_RETRIES - 1 && status.is_server_error() {
                let backoff = INITIAL_BACKOFF_MS * 2u64.pow(attempt);
                warn!(
                    "Discord server error {} (attempt {}/{}), retrying in {}ms",
                    status,
                    attempt + 1,
                    MAX_RETRIES,
                    backoff
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
                continue;
            }

            anyhow::bail!("Discord status {}: {}", status, body);
        }

        return Ok(());
    }

    anyhow::bail!("Discord delivery failed after {} attempts", MAX_RETRIES)
}

pub async fn set_webhook_avatar(client: &reqwest::Client, webhook_url: &str) -> Result<()> {
    match tokio::fs::read("assets/logo.png").await {
        Ok(image_data) => {
            let b64 = general_purpose::STANDARD.encode(&image_data);
            let data_uri = format!("data:image/png;base64,{}", b64);

            let payload = serde_json::json!({ "avatar": data_uri });

            let res = client.patch(webhook_url).json(&payload).send().await?;
            let status = res.status();

            if !status.is_success() {
                let body = res.text().await.unwrap_or_default();
                warn!("Failed to set webhook avatar: {} {}", status, body);
            } else {
                info!("Webhook avatar configured from assets/logo.png");
            }
        }
        Err(e) => {
            warn!("Could not read assets/logo.png: {}", e);
        }
    }

    Ok(())
}
