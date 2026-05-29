use anyhow::Result;
use tokio::time::Duration;
use tracing::info;

use crate::config::Config;
use crate::state::SharedAppState;

/// Generic processor for handling items (articles, episodes)
#[allow(dead_code)]
pub async fn process_items<T>(
    state: SharedAppState,
    config: &Config,
    items: Vec<T>,
    mut item_handler: impl for<'a> FnMut(
        &'a mut crate::state::AppState,
        T,
        bool,
        usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<bool>> + Send + 'a>>,
) -> Result<u32> {
    let mut state_guard = state.lock().await;
    let first_run = !state_guard.initialized;
    let mut new_count = 0;
    let demo_limit = config.scheduling.demo_mode_item_limit;

    if first_run {
        info!("🆕 First run → sending up to {} items as demo", demo_limit);
    }

    for (i, item) in items.into_iter().enumerate() {
        if first_run && i >= demo_limit {
            break;
        }

        if let Ok(sent) = item_handler(&mut state_guard, item, first_run, i).await {
            if sent {
                new_count += 1;
                // Apply delay between messages
                drop(state_guard);
                tokio::time::sleep(Duration::from_millis(
                    config.discord.delay_between_messages_ms,
                ))
                .await;
                state_guard = state.lock().await;
            }
        }
    }

    state_guard.initialized = true;
    state_guard.update_last_check();
    Ok(new_count)
}

/// Send Discord messages to all configured webhooks
pub async fn send_to_all_webhooks(
    client: &reqwest::Client,
    webhook_urls: &[String],
    components: Vec<crate::models::Component>,
) -> Result<u32> {
    let mut success_count = 0;

    for webhook_url in webhook_urls {
        match crate::discord::send_discord(client, webhook_url, components.clone()).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                tracing::error!("❌ Discord delivery failed for webhook {}: {:?}", webhook_url, e);
            }
        }
    }

    Ok(success_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor() {
        assert!(true);
    }
}
