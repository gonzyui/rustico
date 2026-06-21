use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub sources: SourcesConfig,
    pub scheduling: SchedulingConfig,
    pub api: ApiConfig,
    pub messages: MessagesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub webhook_urls: Vec<String>,
    pub delay_between_messages_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    pub ann_rss_urls: Vec<String>,
    pub anilist_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingConfig {
    pub check_interval_minutes: u64,
    pub demo_mode_item_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessagesConfig {
    pub colors: ColorsConfig,
    pub formatting: FormattingConfig,
    #[serde(default)]
    pub demo: Option<serde_yml::Value>,
    #[serde(default)]
    pub errors: Option<serde_yml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorsConfig {
    #[serde(default = "default_ann_color")]
    pub ann: u32,
    #[serde(default = "default_anilist_color")]
    pub anilist: u32,
    #[serde(default = "default_error_color")]
    pub error: u32,
    #[serde(default = "default_success_color")]
    pub success: u32,
}

fn default_ann_color() -> u32 {
    0x1E90FF
}
fn default_anilist_color() -> u32 {
    0x8A2BE2
}
fn default_error_color() -> u32 {
    0xFF0000
}
fn default_success_color() -> u32 {
    0x00FF00
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FormattingConfig {
    pub ann: AnnFormattingConfig,
    pub anilist: AnilistFormattingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnFormattingConfig {
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default = "default_ann_prefix")]
    pub title_prefix: String,
    #[serde(default = "default_true")]
    pub show_timestamp: bool,
    #[serde(default = "default_ann_truncate")]
    pub truncate_description: usize,
    #[serde(default = "default_true")]
    pub show_source: bool,
    pub sections: Vec<SectionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnilistFormattingConfig {
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default = "default_anilist_prefix")]
    pub title_prefix: String,
    #[serde(default = "default_true")]
    pub show_timestamp: bool,
    #[serde(default = "default_anilist_truncate")]
    pub truncate_description: usize,
    #[serde(default = "default_true")]
    pub show_cover: bool,
    #[serde(default = "default_true")]
    pub show_score: bool,
    pub sections: Vec<SectionConfig>,
}

fn default_username() -> String {
    "Rustico".to_string()
}
fn default_ann_prefix() -> String {
    "📰".to_string()
}
fn default_anilist_prefix() -> String {
    "🎬".to_string()
}
fn default_true() -> bool {
    true
}
fn default_ann_truncate() -> usize {
    400
}
fn default_anilist_truncate() -> usize {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionConfig {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub url_field: Option<String>,
    #[serde(default)]
    pub divider: Option<bool>,
    #[serde(default)]
    pub spacing: Option<u8>,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            ann: default_ann_color(),
            anilist: default_anilist_color(),
            error: default_error_color(),
            success: default_success_color(),
        }
    }
}

impl Default for AnnFormattingConfig {
    fn default() -> Self {
        Self {
            username: default_username(),
            title_prefix: default_ann_prefix(),
            show_timestamp: default_true(),
            truncate_description: default_ann_truncate(),
            show_source: default_true(),
            sections: vec![
                SectionConfig {
                    kind: "header".to_string(),
                    format: Some("# {title_prefix} {title}".to_string()),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "link".to_string(),
                    format: Some("[Read full article]({link})".to_string()),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "separator".to_string(),
                    format: None,
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "description".to_string(),
                    format: None,
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "metadata".to_string(),
                    format: Some("-# {source} • {timestamp}".to_string()),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
            ],
        }
    }
}

impl Default for AnilistFormattingConfig {
    fn default() -> Self {
        Self {
            username: default_username(),
            title_prefix: default_anilist_prefix(),
            show_timestamp: default_true(),
            truncate_description: default_anilist_truncate(),
            show_cover: default_true(),
            show_score: default_true(),
            sections: vec![
                SectionConfig {
                    kind: "header".to_string(),
                    format: Some(
                        "# {title_prefix} {title}\n**Episode {episode}** • aired {airing_time}"
                            .to_string(),
                    ),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "link".to_string(),
                    format: Some("[View on AniList]({url})".to_string()),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "thumbnail".to_string(),
                    format: None,
                    url_field: Some("cover_url".to_string()),
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "separator".to_string(),
                    format: None,
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "description".to_string(),
                    format: None,
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "metadata".to_string(),
                    format: Some(
                        "**🎨 Studio**\n{studio}\n\n**⭐ Average Score**\n{score}/100".to_string(),
                    ),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "separator".to_string(),
                    format: None,
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
                SectionConfig {
                    kind: "footer".to_string(),
                    format: Some("-# AniList • {score}/100".to_string()),
                    url_field: None,
                    divider: None,
                    spacing: None,
                },
            ],
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let webhook_urls_raw =
            std::env::var("DISCORD_WEBHOOK_URL").context("Missing DISCORD_WEBHOOK_URL in .env")?;

        let webhook_urls: Vec<String> = webhook_urls_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if webhook_urls.is_empty() {
            bail!("DISCORD_WEBHOOK_URL is empty or invalid");
        }

        for url in &webhook_urls {
            if !url.starts_with("https://discord.com/api/webhooks/") {
                bail!("Invalid Discord webhook URL: {}", url);
            }
        }

        let ann_rss_urls_raw = std::env::var("ANN_RSS_URLS")
            .unwrap_or_else(|_| "https://www.animenewsnetwork.com/all/rss.xml".to_string());

        let ann_rss_urls: Vec<String> = ann_rss_urls_raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let check_interval_minutes: u64 = std::env::var("CHECK_INTERVAL_MINUTES")
            .unwrap_or_else(|_| "15".to_string())
            .parse()
            .context("CHECK_INTERVAL_MINUTES must be a valid number >= 1")?;

        if check_interval_minutes == 0 {
            bail!("CHECK_INTERVAL_MINUTES must be >= 1");
        }

        let delay_between_messages_ms: u64 = std::env::var("DELAY_BETWEEN_MESSAGES_MS")
            .unwrap_or_else(|_| "800".to_string())
            .parse()
            .context("DELAY_BETWEEN_MESSAGES_MS must be a valid number")?;

        let demo_mode_item_limit: usize = std::env::var("DEMO_MODE_ITEM_LIMIT")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .context("DEMO_MODE_ITEM_LIMIT must be a valid number")?;

        let anilist_enabled = std::env::var("ANILIST_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .context("ANILIST_ENABLED must be 'true' or 'false'")?;

        let api_enabled = std::env::var("API_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .context("API_ENABLED must be 'true' or 'false'")?;

        let api_host = std::env::var("API_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let api_port: u16 = std::env::var("API_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .context("API_PORT must be a valid port number")?;

        let messages = Self::load_messages_config("messages.yaml")
            .context("Failed to load messages config")?;

        Ok(Config {
            discord: DiscordConfig {
                webhook_urls,
                delay_between_messages_ms,
            },
            sources: SourcesConfig {
                ann_rss_urls,
                anilist_enabled,
            },
            scheduling: SchedulingConfig {
                check_interval_minutes,
                demo_mode_item_limit,
            },
            api: ApiConfig {
                enabled: api_enabled,
                host: api_host,
                port: api_port,
            },
            messages,
        })
    }

    pub fn load_messages_config(path: &str) -> Result<MessagesConfig> {
        let full_path =
            if path.starts_with("/") || path.starts_with("./") || path.starts_with("../") {
                path.to_string()
            } else {
                format!("config/{}", path)
            };

        if !Path::new(&full_path).exists() {
            bail!("Messages config file not found: {}", full_path);
        }

        let content = std::fs::read_to_string(&full_path)
            .context(format!("Failed to read messages config: {}", full_path))?;

        serde_yml::from_str(&content).context("Invalid YAML in messages config")
    }

    pub fn validate(&self) -> Result<()> {
        if self.discord.webhook_urls.is_empty() {
            bail!("At least one Discord webhook URL is required");
        }

        if self.sources.ann_rss_urls.is_empty() && !self.sources.anilist_enabled {
            bail!("At least one source (ANN RSS or AniList) must be enabled");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_empty_webhooks() {
        let config = Config {
            discord: DiscordConfig {
                webhook_urls: vec![],
                delay_between_messages_ms: 800,
            },
            sources: SourcesConfig {
                ann_rss_urls: vec![],
                anilist_enabled: false,
            },
            scheduling: SchedulingConfig {
                check_interval_minutes: 15,
                demo_mode_item_limit: 3,
            },
            api: ApiConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            messages: MessagesConfig::default(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_no_sources() {
        let config = Config {
            discord: DiscordConfig {
                webhook_urls: vec!["https://discord.com/api/webhooks/123/abc".to_string()],
                delay_between_messages_ms: 800,
            },
            sources: SourcesConfig {
                ann_rss_urls: vec![],
                anilist_enabled: false,
            },
            scheduling: SchedulingConfig {
                check_interval_minutes: 15,
                demo_mode_item_limit: 3,
            },
            api: ApiConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            messages: MessagesConfig::default(),
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config {
            discord: DiscordConfig {
                webhook_urls: vec!["https://discord.com/api/webhooks/123/abc".to_string()],
                delay_between_messages_ms: 800,
            },
            sources: SourcesConfig {
                ann_rss_urls: vec!["https://www.animenewsnetwork.com/all/rss.xml".to_string()],
                anilist_enabled: true,
            },
            scheduling: SchedulingConfig {
                check_interval_minutes: 15,
                demo_mode_item_limit: 3,
            },
            api: ApiConfig {
                enabled: true,
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            messages: MessagesConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_messages_config_not_found() {
        let result = Config::load_messages_config("nonexistent.yaml");
        assert!(result.is_err());
    }
}
