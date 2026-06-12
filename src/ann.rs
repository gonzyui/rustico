use crate::config::Config;
use crate::models::{Component, Container, Separator, TextDisplay};
use crate::processor::send_to_all_webhooks;
use crate::state::SharedAppState;
use crate::utils::clean_html;
use anyhow::Result;
use tracing::{debug, error, info};

/// Holds the data needed to send a single ANN article notification,
/// allowing us to release the mutex before performing I/O.
struct ArticleNotification {
    components: Vec<Component>,
    title: String,
}

pub async fn check_ann(
    state: SharedAppState,
    client: reqwest::Client,
    config: &Config,
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

    // Phase 1: Collect new items under the lock (brief hold)
    let notifications: Vec<ArticleNotification> = {
        let mut state_guard = state.lock().await;
        let first_run = !state_guard.initialized;
        let demo_limit = config.scheduling.demo_mode_item_limit;

        if first_run {
            info!(
                "🆕 [ANN] First run → sending up to {} articles as demo",
                demo_limit
            );
        }

        let mut items = Vec::new();

        for (i, item) in channel.items().iter().take(20).enumerate() {
            if first_run && i >= demo_limit {
                break;
            }

            let guid = item
                .guid()
                .map(|g| g.value().to_string())
                .or_else(|| item.link().map(String::from))
                .unwrap_or_default();

            if guid.is_empty() {
                continue;
            }

            if state_guard.is_seen_ann(&guid) {
                debug!(
                    "⏭️ [ANN] Already seen: {}",
                    item.title().unwrap_or("Unknown")
                );
                continue;
            }

            state_guard.add_seen_ann(guid);

            let title = item.title().unwrap_or("Untitled").to_string();
            let link = item.link().map(String::from).filter(|s| !s.is_empty());

            let raw_desc = item.description().unwrap_or("");
            let cleaned = clean_html(raw_desc);
            let truncate_len = 400;
            let description: String = if cleaned.is_empty() {
                "*No description available.*".to_string()
            } else {
                let truncated: String = cleaned.chars().take(truncate_len).collect();
                if cleaned.chars().count() > truncate_len {
                    format!("{}...", truncated)
                } else {
                    truncated
                }
            };

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

            items.push(ArticleNotification {
                components,
                title: title.clone(),
            });
        }

        items
    };
    // Lock is released here

    // Phase 2: Send notifications without holding the lock
    let mut new_count: u32 = 0;
    for notification in &notifications {
        info!("📤 [ANN] Sending: {}", notification.title);

        match send_to_all_webhooks(
            &client,
            &config.discord.webhook_urls,
            notification.components.clone(),
        )
        .await
        {
            Ok(count) => {
                new_count += count;
            }
            Err(e) => {
                error!("❌ [ANN] Discord delivery failed: {:?}", e);
                let mut state_guard = state.lock().await;
                state_guard.increment_errors();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(
            config.discord.delay_between_messages_ms,
        ))
        .await;
    }

    // Phase 3: Update stats under the lock (brief hold)
    {
        let mut state_guard = state.lock().await;
        for _ in 0..new_count {
            state_guard.increment_articles_sent();
        }
        state_guard.initialized = true;
        state_guard.update_last_check();
    }

    info!("✅ [ANN] Sent {} article(s)", new_count);
    Ok(())
}
