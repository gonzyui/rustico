use crate::models::{DiscordEmbed, DiscordWebhook};
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use tracing::warn;

pub async fn send_discord(
    client: &reqwest::Client,
    webhook_url: &str,
    embed: DiscordEmbed,
) -> Result<()> {
    let payload = DiscordWebhook {
        username: "Rustico".to_string(),
        avatar_url: None,
        embeds: vec![embed],
    };

    let res = client.post(webhook_url).json(&payload).send().await?;
    let status = res.status();

    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        warn!("⚠️ Discord responded with status {}: {}", status, body);
        anyhow::bail!("Discord status {}", status);
    }

    Ok(())
}

pub async fn set_webhook_avatar(
    client: &reqwest::Client,
    webhook_url: &str,
) -> Result<()> {
    match tokio::fs::read("assets/logo.png").await {
        Ok(image_data) => {
            let b64 = general_purpose::STANDARD.encode(&image_data);
            let data_uri = format!("data:image/png;base64,{}", b64);

            let payload = serde_json::json!({ "avatar": data_uri });

            let res = client.patch(webhook_url).json(&payload).send().await?;
            let status = res.status();

            if !status.is_success() {
                let body = res.text().await.unwrap_or_default();
                warn!("⚠️ Failed to set webhook avatar: {} {}", status, body);
            } else {
                tracing::info!("✅ Webhook avatar configured from assets/logo.png");
            }
        }
        Err(e) => {
            warn!("⚠️ Could not read assets/logo.png: {}", e);
        }
    }

    Ok(())
}