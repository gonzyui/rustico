use crate::config::Config;
use crate::models::{AniListResponse, Component, Container, Section, Separator, TextDisplay};
use crate::processor::send_to_all_webhooks;
use crate::sources::Source;
use crate::utils::clean_html;
use anyhow::{Context, Result};
use tracing::{debug, info, warn};

pub struct AnilistSource;

pub struct EpisodeNotification {
    pub components: Vec<Component>,
    pub title: String,
    pub episode: i32,
}

impl Source for AnilistSource {
    type RawItem = crate::models::AiringSchedule;
    type Notification = EpisodeNotification;

    async fn fetch(
        &self,
        client: &reqwest::Client,
        _config: &Config,
    ) -> Result<Vec<Self::RawItem>> {
        info!("[AniList] Fetching recent episodes...");

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
            "[AniList] Raw response (first 200 chars): {}",
            raw_response.chars().take(200).collect::<String>()
        );

        let res: AniListResponse = serde_json::from_str(&raw_response)
            .context("AniList parsing error — see raw response in debug logs")?;

        info!(
            "[AniList] Found {} episode(s) in the time window",
            res.data.page.airing_schedules.len()
        );

        Ok(res.data.page.airing_schedules)
    }

    async fn filter_and_format(
        &self,
        state: &mut crate::state::AppState,
        config: &Config,
        raw_items: Vec<Self::RawItem>,
    ) -> Result<Vec<Self::Notification>> {
        let first_run = !state.initialized;
        let demo_limit = config.scheduling.demo_mode_item_limit;

        if first_run {
            info!(
                "[AniList] First run → sending up to {} episodes as demo",
                demo_limit
            );
        }

        let mut items = Vec::new();

        for (i, schedule) in raw_items.into_iter().enumerate() {
            if first_run && i >= demo_limit {
                break;
            }

            if state.is_seen_anilist(schedule.id) {
                continue;
            }
            state.add_seen_anilist(schedule.id);

            let title = schedule
                .media
                .title
                .english
                .as_deref()
                .or(schedule.media.title.romaji.as_deref())
                .unwrap_or("Unknown Anime")
                .to_string();

            let studio_name = schedule
                .media
                .studios
                .as_ref()
                .and_then(|s| s.nodes.first())
                .map(|n| n.name.as_str())
                .unwrap_or("Unknown Studio");

            let score = schedule.media.average_score.unwrap_or(0);

            let truncate_len = config.messages.formatting.anilist.truncate_description;
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
                    "[AniList] Invalid airing_at={} for episode id={}",
                    airing_ts, schedule.id
                );
                "unknown".to_string()
            };

            let cover_url = schedule
                .media
                .cover_image
                .as_ref()
                .and_then(|c| c.large.clone());

            let mut vars = std::collections::HashMap::new();
            vars.insert(
                "title_prefix",
                config.messages.formatting.anilist.title_prefix.clone(),
            );
            vars.insert("title", title.clone());
            vars.insert("episode", schedule.episode.to_string());
            vars.insert(
                "airing_time",
                if config.messages.formatting.anilist.show_timestamp {
                    airing_display.clone()
                } else {
                    "".to_string()
                },
            );
            vars.insert("url", schedule.media.site_url.clone());
            vars.insert("cover_url", cover_url.clone().unwrap_or_default());
            vars.insert("studio", studio_name.to_string());
            vars.insert(
                "score",
                if config.messages.formatting.anilist.show_score {
                    score.to_string()
                } else {
                    "".to_string()
                },
            );

            let mut accumulated_text = Vec::new();
            let mut container_components = Vec::new();

            for (idx, sec) in config
                .messages
                .formatting
                .anilist
                .sections
                .iter()
                .enumerate()
            {
                match sec.kind.as_str() {
                    "header" | "description" | "metadata" | "footer" => {
                        if let Some(ref fmt) = sec.format {
                            let rendered = crate::utils::render_template(fmt, &vars);
                            if !rendered.is_empty() {
                                accumulated_text
                                    .push(Component::TextDisplay(TextDisplay::new(rendered)));
                            }
                        } else if sec.kind == "description" {
                            if !description.is_empty() {
                                accumulated_text
                                    .push(Component::TextDisplay(TextDisplay::new(&description)));
                            }
                        }
                    }
                    "link" => {
                        let url_val = vars.get("url");
                        if let Some(val) = url_val {
                            if !val.is_empty() {
                                if let Some(ref fmt) = sec.format {
                                    let rendered = crate::utils::render_template(fmt, &vars);
                                    if !rendered.is_empty() {
                                        accumulated_text.push(Component::TextDisplay(
                                            TextDisplay::new(rendered),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    "separator" => {
                        if !accumulated_text.is_empty() {
                            container_components.append(&mut accumulated_text);
                        }
                        let divider = sec.divider.unwrap_or_else(|| {
                            let mut followed_by_footer_or_metadata = true;
                            for next_sec in &config.messages.formatting.anilist.sections[idx + 1..]
                            {
                                if next_sec.kind != "metadata" && next_sec.kind != "footer" {
                                    followed_by_footer_or_metadata = false;
                                    break;
                                }
                            }
                            !followed_by_footer_or_metadata
                        });
                        container_components
                            .push(Component::Separator(Separator::new(divider, false)));
                    }
                    "thumbnail" => {
                        let mut thumb_url = None;
                        if config.messages.formatting.anilist.show_cover {
                            if let Some(ref field) = sec.url_field {
                                if let Some(val) = vars.get(field.as_str()) {
                                    if !val.is_empty() {
                                        thumb_url = Some(val.clone());
                                    }
                                }
                            }
                        }
                        if let Some(url) = thumb_url {
                            let text_comps = std::mem::take(&mut accumulated_text);
                            container_components
                                .push(Component::Section(Section::with_thumbnail(text_comps, url)));
                        } else {
                            container_components.append(&mut accumulated_text);
                        }
                    }
                    _ => {}
                }
            }
            if !accumulated_text.is_empty() {
                container_components.append(&mut accumulated_text);
            }

            let components = vec![Component::Container(Container::new(
                Some(config.messages.colors.anilist),
                container_components,
            ))];

            items.push(EpisodeNotification {
                components,
                title: title.clone(),
                episode: schedule.episode,
            });
        }

        Ok(items)
    }

    async fn send(
        &self,
        client: &reqwest::Client,
        config: &Config,
        notification: &Self::Notification,
    ) -> Result<u32> {
        info!(
            "[AniList] Sending: {} EP{}",
            notification.title, notification.episode
        );

        send_to_all_webhooks(
            client,
            &config.discord.webhook_urls,
            &config.messages.formatting.anilist.username,
            notification.components.clone(),
        )
        .await
    }

    async fn update_state(
        &self,
        state: &mut crate::state::AppState,
        success_count: u32,
    ) -> Result<()> {
        for _ in 0..success_count {
            state.increment_episodes_sent();
        }
        info!("[AniList] Sent {} episode(s)", success_count);
        Ok(())
    }
}
