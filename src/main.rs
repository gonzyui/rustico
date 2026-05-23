mod anilist;
mod ann;
mod discord;
mod models;

use anyhow::{Context, Result};
use std::sync::Arc;
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
        .with_max_level(tracing::Level::DEBUG)
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
    let client = Arc::new(reqwest::Client::new());

    info!("🚀 Starting Rustico");
    info!(
        "   Webhook configured: {}...",
        &webhook_url[..50.min(webhook_url.len())]
    );
    info!("   Interval: {} min", interval_min);

    info!("⏱️ Executing initial pass...");

    if let Err(e) = check_ann(state.clone(), client.clone(), &webhook_url, &rss_url).await {
        error!("ANN Error: {:?}", e);
    }

    if let Err(e) = check_anilist(state.clone(), client.clone(), &webhook_url).await {
        error!("AniList Error: {:?}", e);
    }

    let sched = JobScheduler::new().await?;
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
                info!("⏰ Scheduler tick!");

                if let Err(e) = check_ann(state.clone(), client.clone(), &webhook, &rss).await {
                    error!("ANN Error: {:?}", e);
                }

                if let Err(e) = check_anilist(state, client, &webhook).await {
                    error!("AniList Error: {:?}", e);
                }
            })
        })?)
        .await?;

    sched.start().await?;
    info!("✅ Scheduler started, waiting...");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
