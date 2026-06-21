use crate::config::Config;
use crate::models::{Component, Container, Separator, TextDisplay};
use crate::processor::send_to_all_webhooks;
use crate::sources::Source;
use crate::utils::clean_html;
use anyhow::Result;
use tracing::{debug, info};

pub struct AnnSource {
    pub rss_url: String,
}

pub struct ArticleNotification {
    pub components: Vec<Component>,
    pub title: String,
}

impl Source for AnnSource {
    type RawItem = rss::Item;
    type Notification = ArticleNotification;

    async fn fetch(
        &self,
        client: &reqwest::Client,
        _config: &Config,
    ) -> Result<Vec<Self::RawItem>> {
        info!("[ANN] Fetching RSS feed: {}", self.rss_url);

        let response = client.get(&self.rss_url).send().await?.bytes().await?;
        info!("[ANN] Received {} bytes", response.len());

        let channel = rss::Channel::read_from(&response[..])?;
        info!("[ANN] Found {} articles in the feed", channel.items().len());

        Ok(channel.items().to_vec())
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
                "[ANN] First run → sending up to {} articles as demo",
                demo_limit
            );
        }

        let mut items = Vec::new();

        for (i, item) in raw_items.iter().take(20).enumerate() {
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

            if state.is_seen_ann(&guid) {
                debug!("[ANN] Already seen: {}", item.title().unwrap_or("Unknown"));
                continue;
            }

            state.add_seen_ann(guid);

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

            for (idx, sec) in config.messages.formatting.ann.sections.iter().enumerate() {
                match sec.kind.as_str() {
                    "header" | "description" | "metadata" | "footer" => {
                        if let Some(ref fmt) = sec.format {
                            let rendered = crate::utils::render_template(fmt, &vars);
                            if !rendered.is_empty() {
                                accumulated_text
                                    .push(Component::TextDisplay(TextDisplay::new(rendered)));
                            }
                        } else if sec.kind == "description" && !description.is_empty() {
                            accumulated_text
                                .push(Component::TextDisplay(TextDisplay::new(&description)));
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

        Ok(items)
    }

    async fn send(
        &self,
        client: &reqwest::Client,
        config: &Config,
        notification: &Self::Notification,
    ) -> Result<u32> {
        info!("[ANN] Sending: {}", notification.title);

        send_to_all_webhooks(
            client,
            &config.discord.webhook_urls,
            &config.messages.formatting.ann.username,
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
            state.increment_articles_sent();
        }
        info!("[ANN] Sent {} article(s)", success_count);
        Ok(())
    }
}
