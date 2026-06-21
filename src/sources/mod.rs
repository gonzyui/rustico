use crate::config::Config;
use crate::state::SharedAppState;
use anyhow::Result;

pub mod anilist;
pub mod ann;

/// Represents a generic data source that fetches, filters, formats, and sends updates.
pub trait Source {
    /// The raw item type fetched from the source.
    type RawItem;

    /// The notification item type after formatting.
    type Notification;

    /// Fetches raw items from the remote source.
    async fn fetch(&self, client: &reqwest::Client, config: &Config) -> Result<Vec<Self::RawItem>>;

    /// Filters and formats raw items under the application state lock.
    async fn filter_and_format(
        &self,
        state: &mut crate::state::AppState,
        config: &Config,
        raw_items: Vec<Self::RawItem>,
    ) -> Result<Vec<Self::Notification>>;

    /// Sends a single formatted notification to target webhooks.
    async fn send(
        &self,
        client: &reqwest::Client,
        config: &Config,
        notification: &Self::Notification,
    ) -> Result<u32>;

    /// Updates the state stats after successfully sending notifications.
    async fn update_state(
        &self,
        state: &mut crate::state::AppState,
        success_count: u32,
    ) -> Result<()>;

    /// Executes the full check cycle: fetch, filter/format, send, and update state.
    async fn check(
        &self,
        state: SharedAppState,
        client: reqwest::Client,
        config: &Config,
    ) -> Result<()> {
        let raw_items = self.fetch(&client, config).await?;

        let notifications = {
            let mut state_guard = state.write().await;
            self.filter_and_format(&mut state_guard, config, raw_items).await?
        };

        let mut success_count = 0;
        for notification in &notifications {
            match self.send(&client, config, notification).await {
                Ok(count) => success_count += count,
                Err(e) => {
                    tracing::error!("Delivery failed: {:?}", e);
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
            self.update_state(&mut state_guard, success_count).await?;
            state_guard.initialized = true;
            state_guard.update_last_check();
        }

        Ok(())
    }
}
