mod anilist;
mod ann;
mod discord;
mod models;
mod utils;

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::anilist::check_anilist;
use crate::ann::check_ann;
use crate::models::AppState;

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

    let webhook_url =
        std::env::var("DISCORD_WEBHOOK_URL").context("Missing DISCORD_WEBHOOK_URL in .env")?;

    let rss_url = std::env::var("ANN_RSS_URL")
        .unwrap_or_else(|_| "https://www.animenewsnetwork.com/all/rss.xml".to_string());

    let interval_min: u64 = std::env::var("CHECK_INTERVAL_MINUTES")
        .unwrap_or_else(|_| "15".to_string())
        .parse()
        .unwrap_or(15);

    let state = Arc::new(Mutex::new(AppState::default()));

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!(
            "Rustico/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/gonzyui/rustico)"
        ))
        .build()
        .context("Failed to build reqwest client")?;

    info!("🚀 Starting Rustico v{}", env!("CARGO_PKG_VERSION"));
    info!(
        "   Webhook configured: {}...",
        &webhook_url[..50.min(webhook_url.len())]
    );
    info!("   Interval: {} min", interval_min);

    info!("⏱️ Executing initial pass...");

    if let Err(e) = crate::discord::set_webhook_avatar(&client, &webhook_url).await {
        error!("Webhook avatar configuration error: {:?}", e);
    }

    if let Err(e) = check_ann(state.clone(), client.clone(), &webhook_url, &rss_url).await {
        error!("ANN Error: {:?}", e);
    }

    if let Err(e) = check_anilist(state.clone(), client.clone(), &webhook_url).await {
        error!("AniList Error: {:?}", e);
    }

    state.lock().await.initialized = true;
    info!("✅ Initial pass completed — state initialized");

    let mut sched = JobScheduler::new().await?;
    let cron_expr = format!("0 */{} * * * *", interval_min);
    info!("⏰ Cron configured: '{}'", cron_expr);

    let state_clone = state.clone();
    let webhook_clone = webhook_url.clone();
    let rss_clone = rss_url.clone();
    let client_clone = client.clone();

    sched
        .add(Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
            let state = state_clone.clone();
            let webhook = webhook_clone.clone();
            let rss = rss_clone.clone();
            let client = client_clone.clone();
            Box::pin(async move {
                info!("⏰ Scheduled tick");
                if let Err(e) = check_ann(state.clone(), client.clone(), &webhook, &rss).await {
                    error!("ANN Error: {:?}", e);
                }
                if let Err(e) = check_anilist(state.clone(), client.clone(), &webhook).await {
                    error!("AniList Error: {:?}", e);
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
    info!("👋 Bye!");

    Ok(())
}