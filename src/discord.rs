use crate::models::{DiscordEmbed, DiscordWebhook};
use anyhow::Result;
use std::sync::Arc;
use tracing::warn;

pub async fn send_discord(
    client: &Arc<reqwest::Client>,
    webhook_url: &str,
    embed: DiscordEmbed,
) -> Result<()> {
    let payload = DiscordWebhook {
        username: "Rustico".to_string(),
        avatar_url: "https://cdn-icons-png.flaticon.com/512/2111/2111646.png".to_string(),
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
