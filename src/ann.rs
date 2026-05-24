use crate::discord::send_discord;
use crate::models::{AppState, Component, Container, Separator, TextDisplay};
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
        let link = item.link().map(String::from).filter(|s| !s.is_empty());

        let raw_desc = item.description().unwrap_or("");
        let cleaned = clean_html(raw_desc);
        let description: String = if cleaned.is_empty() {
            "*No description available.*".to_string()
        } else {
            let truncated: String = cleaned.chars().take(400).collect();
            if cleaned.chars().count() > 400 {
                format!("{}...", truncated)
            } else {
                truncated
            }
        };

        info!("📤 [ANN] Sending: {}", title);

        let header = match &link {
            Some(url) => format!("# 📰 {}\n[Read full article]({})", title, url),
            None => format!("# 📰 {}", title),
        };

        let now_relative = format!("<t:{}:R>", chrono::Utc::now().timestamp());

        let container_components = vec![
            Component::TextDisplay(TextDisplay::new(header)),
            Component::Separator(Separator::new(true, false)),
            Component::TextDisplay(TextDisplay::new(description)),
            Component::Separator(Separator::new(false, false)),
            Component::TextDisplay(TextDisplay::new(format!(
                "-# Anime News Network • {}",
                now_relative
            ))),
        ];

        let components = vec![Component::Container(Container::new(
            Some(0x1E90FF),
            container_components,
        ))];

        if let Err(e) = send_discord(&client, webhook_url, components).await {
            error!("❌ [ANN] Discord delivery failed: {:?}", e);
        } else {
            new_count += 1;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    }

    info!("✅ [ANN] Sent {} article(s)", new_count);
    Ok(())
}