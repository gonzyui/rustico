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
    info!("[ANN] Fetching RSS feed: {}", rss_url);

    let response = client.get(rss_url).send().await?.bytes().await?;
    info!("[ANN] Received {} bytes", response.len());

    let channel = rss::Channel::read_from(&response[..])?;
    info!("[ANN] Found {} articles in the feed", channel.items().len());

    let notifications: Vec<ArticleNotification> = {
        let mut state_guard = state.write().await;
        let first_run = !state_guard.initialized;
        let demo_limit = config.scheduling.demo_mode_item_limit;

        if first_run {
            info!(
                "[ANN] First run → sending up to {} articles as demo",
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
                debug!("[ANN] Already seen: {}", item.title().unwrap_or("Unknown"));
                continue;
            }

            state_guard.add_seen_ann(guid);

            let title = item.title().unwrap_or("Untitled").to_string();
            let link = item.link().map(String::from).filter(|s| !s.is_empty());

            let raw_desc = item.description().unwrap_or("");
            let cleaned = clean_html(raw_desc);
            let truncate_len = config.messages.formatting.ann.truncate_description;
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

            let mut vars = std::collections::HashMap::new();
            vars.insert(
                "title_prefix",
                config.messages.formatting.ann.title_prefix.clone(),
            );
            vars.insert("title", title.clone());
            vars.insert("link", link.clone().unwrap_or_default());
            vars.insert("description", description.clone());
            vars.insert(
                "source",
                if config.messages.formatting.ann.show_source {
                    "Anime News Network".to_string()
                } else {
                    "".to_string()
                },
            );
            vars.insert(
                "timestamp",
                if config.messages.formatting.ann.show_timestamp {
                    format!("<t:{}:R>", chrono::Utc::now().timestamp())
                } else {
                    "".to_string()
                },
            );

            let mut accumulated_text = Vec::new();
            let mut container_components = Vec::new();
            let _sections_len = config.messages.formatting.ann.sections.len();

            for (idx, sec) in config.messages.formatting.ann.sections.iter().enumerate() {
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
                        let link_val = vars.get("link");
                        if let Some(val) = link_val {
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
                            for next_sec in &config.messages.formatting.ann.sections[idx + 1..] {
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
                        if let Some(ref field) = sec.url_field {
                            if let Some(val) = vars.get(field.as_str()) {
                                if !val.is_empty() {
                                    thumb_url = Some(val.clone());
                                }
                            }
                        }
                        if let Some(url) = thumb_url {
                            let text_comps = std::mem::take(&mut accumulated_text);
                            container_components.push(Component::Section(
                                crate::models::Section::with_thumbnail(text_comps, url),
                            ));
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
                Some(config.messages.colors.ann),
                container_components,
            ))];

            items.push(ArticleNotification {
                components,
                title: title.clone(),
            });
        }

        items
    };

    let mut new_count: u32 = 0;
    for notification in &notifications {
        info!("[ANN] Sending: {}", notification.title);

        match send_to_all_webhooks(
            &client,
            &config.discord.webhook_urls,
            &config.messages.formatting.ann.username,
            notification.components.clone(),
        )
        .await
        {
            Ok(count) => {
                new_count += count;
            }
            Err(e) => {
                error!("[ANN] Discord delivery failed: {:?}", e);
                let mut state_guard = state.write().await;
                state_guard.increment_errors();
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(
            config.discord.delay_between_messages_ms,
        ))
        .await;
    }

    {
        let mut state_guard = state.write().await;
        for _ in 0..new_count {
            state_guard.increment_articles_sent();
        }
        state_guard.initialized = true;
        state_guard.update_last_check();
    }

    info!("[ANN] Sent {} article(s)", new_count);
    Ok(())
}
