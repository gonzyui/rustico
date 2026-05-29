mod anilist;
mod ann;
mod api;
mod config;
mod discord;
mod models;
mod processor;
mod state;
mod utils;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::anilist::check_anilist;
use crate::ann::check_ann;
use crate::api::start_health_api;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .parse_lossy("info,rustico=debug"),
        )
        .init();

    // Load and validate configuration
    let config = Config::from_env().context("Configuration error")?;
    config.validate().context("Configuration validation failed")?;

    info!("🚀 Starting Rustico v{}", env!("CARGO_PKG_VERSION"));
    info!("📝 Configuration:");
    info!("   Webhooks: {} webhook(s) configured", config.discord.webhook_urls.len());
    info!("   ANN RSS: {} feed(s)", config.sources.ann_rss_urls.len());
    info!("   AniList: {}", if config.sources.anilist_enabled { "enabled" } else { "disabled" });
    info!("   API: {}", if config.api.enabled { "enabled" } else { "disabled" });
    info!("   Interval: {} min", config.scheduling.check_interval_minutes);
    info!("   Delay between messages: {} ms", config.discord.delay_between_messages_ms);

    // Load or create state
    let app_state = AppState::load();
    let shared_state = Arc::new(Mutex::new(app_state.clone()));

    // Build HTTP client
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(10))
        .user_agent(concat!(
            "Rustico/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/gonzyui/rustico)"
        ))
        .build()
        .context("Failed to build reqwest client")?;

    info!("⏱️ Executing initial pass...");

    // Set webhook avatar
    for webhook_url in &config.discord.webhook_urls {
        if let Err(e) = crate::discord::set_webhook_avatar(&client, webhook_url).await {
            error!("Webhook avatar configuration error: {:?}", e);
        }
    }

    // Check sources
    for rss_url in &config.sources.ann_rss_urls {
        if let Err(e) = check_ann(shared_state.clone(), client.clone(), &config, rss_url).await {
            error!("ANN Error: {:?}", e);
            {
                let mut state = shared_state.lock().await;
                state.increment_errors();
            }
        }
    }

    if config.sources.anilist_enabled {
        if let Err(e) = check_anilist(shared_state.clone(), client.clone(), &config).await {
            error!("AniList Error: {:?}", e);
            {
                let mut state = shared_state.lock().await;
                state.increment_errors();
            }
        }
    }

    {
        let mut state = shared_state.lock().await;
        state.initialized = true;
        if let Err(e) = state.save() {
            error!("Failed to save state: {:?}", e);
        }
    }

    info!("✅ Initial pass completed — state initialized");

    // Start health API in background
    let api_config = config.clone();
    let api_state = shared_state.clone();
    tokio::spawn(async move {
        if let Err(e) = start_health_api(&api_config, api_state).await {
            error!("Health API error: {:?}", e);
        }
    });

    // Setup scheduler
    let mut sched = JobScheduler::new().await?;
    let cron_expr = format!("0 */{} * * * *", config.scheduling.check_interval_minutes);
    info!("⏰ Cron configured: '{}'", cron_expr);

    let state_clone = shared_state.clone();
    let webhook_clone = config.discord.webhook_urls.clone();
    let rss_clone = config.sources.ann_rss_urls.clone();
    let client_clone = client.clone();
    let config_clone = config.clone();
    let anilist_enabled = config.sources.anilist_enabled;

    sched
        .add(Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
            let state = state_clone.clone();
            let _webhooks = webhook_clone.clone();
            let rss_urls = rss_clone.clone();
            let client = client_clone.clone();
            let cfg = config_clone.clone();
            let anilist_en = anilist_enabled;

            Box::pin(async move {
                info!("⏰ Scheduled tick");

                for rss_url in &rss_urls {
                    if let Err(e) = check_ann(state.clone(), client.clone(), &cfg, rss_url).await {
                        error!("ANN Error: {:?}", e);
                        {
                            let mut s = state.lock().await;
                            s.increment_errors();
                        }
                    }
                }

                if anilist_en {
                    if let Err(e) = check_anilist(state.clone(), client.clone(), &cfg).await {
                        error!("AniList Error: {:?}", e);
                        {
                            let mut s = state.lock().await;
                            s.increment_errors();
                        }
                    }
                }

                // Save state after each check
                let s = state.lock().await;
                if let Err(e) = (*s).clone().save() {
                    error!("Failed to save state: {:?}", e);
                }
            })
        })?)
        .await?;

    sched.start().await?;
    info!("✅ Scheduler started — press Ctrl+C to stop");

    tokio::signal::ctrl_c()
        .await
        .context("Failed to listen for Ctrl+C")?;

    info!("🛑 Shutdown signal received, stopping scheduler...");
    if let Err(e) = sched.shutdown().await {
        error!("Error during scheduler shutdown: {:?}", e);
    }

    // Save state before exit
    {
        let state = shared_state.lock().await;
        if let Err(e) = (*state).clone().save() {
            error!("Failed to save state before exit: {:?}", e);
        }
    }

    info!("👋 Bye!");

    Ok(())
}