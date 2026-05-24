use crate::discord::send_discord;
use crate::models::{AppState, DiscordEmbed, DiscordFooter};
use crate::utils::clean_html;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

pub async fn check_ann(
    state: Arc<Mutex<AppState>>,
    client: reqwest::Client,
    webhook_url: &str,
    rss_url: &str,
) -> Result<()> {
    info!("🔍 [ANN] Fetching RSS feed: {}", rss_url);

    let response = client.get(rss_url).send().await?.bytes().await?;
    info!("📥 [ANN] Received {} bytes", response.len());

    let channel = rss::Channel::read_from(&response[..])?;
    info!(
        "📰 [ANN] Found {} articles in the feed",
        channel.items().len()
    );

    let mut state_guard = state.lock().await;
    let mut new_count = 0;
    let first_run = !state_guard.initialized;

    if first_run {
        info!("🆕 [ANN] First run → sending the 3 most recent articles as demo");
    }

    for (i, item) in channel.items().iter().take(20).enumerate() {
        let guid = item
            .guid()
            .map(|g| g.value().to_string())
            .or_else(|| item.link().map(String::from))
            .unwrap_or_default();

        if guid.is_empty() {
            continue;
        }

        if state_guard.seen_ann.contains(&guid) {
            debug!(
                "⏭️ [ANN] Already seen: {}",
                item.title().unwrap_or("Unknown")
            );
            continue;
        }

        state_guard.seen_ann.insert(guid.clone());

        if first_run && i >= 3 {
            continue;
        }

        let title = item.title().unwrap_or("Untitled").to_string();
        let link = item.link().map(String::from);

        let raw_desc = item.description().unwrap_or("");
        let cleaned = clean_html(raw_desc);
        let description: String = cleaned.chars().take(300).collect();

        info!("📤 [ANN] Sending: {}", title);

        let embed = DiscordEmbed {
            title: format!("📰 {}", title),
            description,
            url: link.filter(|s| !s.is_empty()),
            color: 0x1E90FF,
            footer: DiscordFooter {
                text: "Anime News Network".to_string(),
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            thumbnail: None,
            fields: vec![],
        };

        if let Err(e) = send_discord(&client, webhook_url, embed).await {
            error!("❌ [ANN] Discord delivery failed: {:?}", e);
        } else {
            new_count += 1;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    }

    info!("✅ [ANN] Sent {} article(s)", new_count);
    Ok(())
}
