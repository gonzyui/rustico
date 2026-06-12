use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub discord: DiscordConfig,
    pub sources: SourcesConfig,
    pub scheduling: SchedulingConfig,
    pub api: ApiConfig,
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

impl Config {
    /// Load configuration from environment variables with validation
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

        // Validate webhook URLs
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
        })
    }

    /// Load configuration from a YAML file (for advanced message templates)
    #[allow(dead_code)]
    pub fn load_messages_config(path: &str) -> Result<serde_yml::Value> {
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
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_messages_config_not_found() {
        let result = Config::load_messages_config("nonexistent.yaml");
        assert!(result.is_err());
    }
}
