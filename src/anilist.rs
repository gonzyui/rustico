use crate::discord::send_discord;
use crate::models::{
    AniListResponse, AppState, DiscordEmbed, DiscordField, DiscordFooter, DiscordImage,
};
use crate::utils::clean_html;
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub async fn check_anilist(
    state: Arc<Mutex<AppState>>,
    client: reqwest::Client,
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
                        description(asHtml: false)
                        averageScore
                        studios(isMain: true) {
                            nodes { name }
                        }
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
            .as_deref()
            .or(schedule.media.title.romaji.as_deref())
            .unwrap_or("Unknown Anime");

        let studio_name = schedule
            .media
            .studios
            .as_ref()
            .and_then(|s| s.nodes.first())
            .map(|n| n.name.as_str())
            .unwrap_or("Unknown Studio");

        let score = schedule.media.average_score.unwrap_or(0);

        let description = match schedule.media.description.as_deref() {
            Some(d) if !d.is_empty() => {
                let cleaned = clean_html(d);
                if cleaned.chars().count() > 200 {
                    let mut s: String = cleaned.chars().take(200).collect();
                    s.push_str("...");
                    s
                } else {
                    cleaned
                }
            }
            _ => String::new(),
        };

        let timestamp = match chrono::DateTime::from_timestamp(schedule.airing_at, 0) {
            Some(dt) => dt.to_rfc3339(),
            None => {
                warn!(
                    "⚠️ [AniList] Invalid airing_at={} for episode id={}, using now()",
                    schedule.airing_at, schedule.id
                );
                chrono::Utc::now().to_rfc3339()
            }
        };

        info!("📤 [AniList] Sending: {} EP{}", title, schedule.episode);

        let embed = DiscordEmbed {
            title: format!("🎬 {} — Episode {}", title, schedule.episode),
            description,
            url: Some(schedule.media.site_url.clone()),
            fields: vec![
                DiscordField {
                    name: "Studio".to_string(),
                    value: studio_name.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "Average Score".to_string(),
                    value: format!("{}/100", score),
                    inline: true,
                },
            ],
            color: 0x8A2BE2,
            footer: DiscordFooter {
                text: format!("AniList • {}/100", score),
            },
            timestamp,
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