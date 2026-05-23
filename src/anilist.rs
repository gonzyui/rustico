use crate::discord::send_discord;
use crate::models::{AniListResponse, AppState, DiscordEmbed, DiscordFooter, DiscordImage};
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

pub async fn check_anilist(
    state: Arc<Mutex<AppState>>,
    client: Arc<reqwest::Client>,
    webhook_url: &str,
) -> Result<()> {
    info!("🔍 [AniList] Fetching recent episodes...");

    let now = chrono::Utc::now().timestamp();
    let window_start = now - 24 * 3600;

    let query = r#"
        query ($from: Int, $to: Int) {
            Page(perPage: 50) {
                airingSchedules(airingAt_greater: $from, airingAt_lesser: $to, sort: TIME_DESC) {
                    id
                    episode
                    airingAt
                    media {
                        title { romaji english }
                        siteUrl
                        coverImage { large }
                    }
                }
            }
        }
    "#;

    let body = serde_json::json!({
        "query": query,
        "variables": { "from": window_start, "to": now }
    });

    let raw_response = client
        .post("https://graphql.anilist.co")
        .json(&body)
        .send()
        .await?
        .text()
        .await?;

    debug!(
        "📥 [AniList] Raw response (first 200 chars): {}",
        raw_response.chars().take(200).collect::<String>()
    );

    let res: AniListResponse = serde_json::from_str(&raw_response)
        .context("AniList parsing error — see raw response in debug logs")?;

    info!(
        "🎬 [AniList] Found {} episode(s) in the time window",
        res.data.page.airing_schedules.len()
    );

    let mut state_guard = state.lock().await;
    let first_run = !state_guard.initialized;
    let mut new_count = 0;

    if first_run {
        info!("🆕 [AniList] First run → sending the 3 most recent episodes as demo");
    }

    for (i, schedule) in res.data.page.airing_schedules.iter().enumerate() {
        if state_guard.seen_anilist.contains(&schedule.id) {
            continue;
        }
        state_guard.seen_anilist.insert(schedule.id);

        if first_run && i >= 3 {
            continue;
        }

        let title = schedule
            .media
            .title
            .english
            .clone()
            .or(schedule.media.title.romaji.clone())
            .unwrap_or_else(|| "Unknown Anime".to_string());

        info!("📤 [AniList] Sending: {} EP{}", title, schedule.episode);

        let embed = DiscordEmbed {
            title: format!("🎬 {} — Episode {}", title, schedule.episode),
            description: "New episode available!".to_string(),
            url: schedule.media.site_url.clone(),
            color: 0x8A2BE2, // BlueViolet
            footer: DiscordFooter {
                text: "AniList".to_string(),
            },
            timestamp: chrono::DateTime::from_timestamp(schedule.airing_at, 0)
                .unwrap_or_else(chrono::Utc::now)
                .to_rfc3339(),
            thumbnail: schedule
                .media
                .cover_image
                .as_ref()
                .and_then(|c| c.large.clone())
                .map(|url| DiscordImage { url }),
        };

        if let Err(e) = send_discord(&client, webhook_url, embed).await {
            error!("❌ [AniList] Discord delivery failed: {:?}", e);
        } else {
            new_count += 1;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    }

    state_guard.initialized = true;

    info!("✅ [AniList] Sent {} episode(s)", new_count);
    Ok(())
}
