use crate::config::Config;
use crate::models::{
    AniListResponse, Component, Container, Section, Separator, TextDisplay,
};
use crate::processor::send_to_all_webhooks;
use crate::state::SharedAppState;
use crate::utils::clean_html;
use anyhow::{Context, Result};
use tracing::{debug, error, info, warn};

pub async fn check_anilist(
    state: SharedAppState,
    client: reqwest::Client,
    config: &Config,
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
    let demo_limit = config.scheduling.demo_mode_item_limit;

    if first_run {
        info!("🆕 [AniList] First run → sending up to {} episodes as demo", demo_limit);
    }

    for (i, schedule) in res.data.page.airing_schedules.iter().enumerate() {
        if first_run && i >= demo_limit {
            break;
        }

        if state_guard.is_seen_anilist(schedule.id) {
            continue;
        }
        state_guard.add_seen_anilist(schedule.id);

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

        let truncate_len = 300;
        let description = match schedule.media.description.as_deref() {
            Some(d) if !d.is_empty() => {
                let cleaned = clean_html(d);
                if cleaned.chars().count() > truncate_len {
                    let mut s: String = cleaned.chars().take(truncate_len).collect();
                    s.push_str("...");
                    s
                } else {
                    cleaned
                }
            }
            _ => "*No description available.*".to_string(),
        };

        let airing_ts = schedule.airing_at;
        let airing_display = if airing_ts > 0 {
            format!("<t:{}:R>", airing_ts)
        } else {
            warn!(
                "⚠️ [AniList] Invalid airing_at={} for episode id={}",
                airing_ts, schedule.id
            );
            "unknown".to_string()
        };

        let cover_url = schedule
            .media
            .cover_image
            .as_ref()
            .and_then(|c| c.large.clone());

        info!("📤 [AniList] Sending: {} EP{}", title, schedule.episode);

        let header_text = format!(
            "# 🎬 {}\n**Episode {}** • aired {}\n[View on AniList]({})",
            title, schedule.episode, airing_display, schedule.media.site_url
        );

        let info_text = format!(
            "**🎨 Studio**\n{}\n\n**⭐ Average Score**\n{}/100",
            studio_name, score
        );

        let mut container_components: Vec<Component> = Vec::new();

        if let Some(ref url) = cover_url {
            container_components.push(Component::Section(Section::with_thumbnail(
                vec![Component::TextDisplay(TextDisplay::new(header_text))],
                url.clone(),
            )));
        } else {
            container_components.push(Component::TextDisplay(TextDisplay::new(header_text)));
        }

        container_components.push(Component::Separator(Separator::new(true, false)));
        container_components.push(Component::TextDisplay(TextDisplay::new(description)));
        container_components.push(Component::Separator(Separator::new(true, false)));
        container_components.push(Component::TextDisplay(TextDisplay::new(info_text)));
        container_components.push(Component::Separator(Separator::new(false, false)));
        container_components.push(Component::TextDisplay(TextDisplay::new(format!(
            "-# AniList • {}/100",
            score
        ))));

        let components = vec![Component::Container(Container::new(
            Some(0x8A2BE2),
            container_components,
        ))];

        match send_to_all_webhooks(&client, &config.discord.webhook_urls, components).await {
            Ok(count) => {
                new_count += count as u32;
                state_guard.increment_episodes_sent();
            }
            Err(e) => {
                error!("❌ [AniList] Discord delivery failed: {:?}", e);
                state_guard.increment_errors();
            }
        }

        // Release lock before sleeping
        drop(state_guard);
        tokio::time::sleep(tokio::time::Duration::from_millis(
            config.discord.delay_between_messages_ms,
        ))
        .await;
        state_guard = state.lock().await;
    }

    state_guard.initialized = true;
    state_guard.update_last_check();

    info!("✅ [AniList] Sent {} episode(s)", new_count);
    Ok(())
}