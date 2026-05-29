use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub seen_ann: HashSet<String>,
    pub seen_anilist: HashSet<i64>,
    pub initialized: bool,
    #[serde(default)]
    pub stats: Stats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    pub total_articles_sent: u64,
    pub total_episodes_sent: u64,
    pub total_errors: u64,
    pub last_check: Option<String>,
}

pub const STATE_FILE: &str = "data/rustico_state.yaml";

impl AppState {
    /// Load state from YAML file, or create a new empty state
    pub fn load() -> Self {
        match std::fs::read_to_string(STATE_FILE) {
            Ok(content) => match serde_yaml::from_str::<AppState>(&content) {
                Ok(state) => {
                    info!("✅ State loaded from {}", STATE_FILE);
                    state
                }
                Err(e) => {
                    warn!("⚠️ Failed to parse {}: {}, starting fresh", STATE_FILE, e);
                    Self::default()
                }
            },
            Err(e) => {
                debug!("📝 State file not found ({}), starting fresh", e);
                Self::default()
            }
        }
    }

    /// Save state to YAML file
    pub fn save(&self) -> Result<()> {
        let yaml = serde_yaml::to_string(self).context("Failed to serialize state to YAML")?;
        std::fs::write(STATE_FILE, yaml).context(format!("Failed to write {}", STATE_FILE))?;
        debug!("💾 State saved to {}", STATE_FILE);
        Ok(())
    }

    pub fn add_seen_ann(&mut self, guid: String) {
        self.seen_ann.insert(guid);
    }

    pub fn add_seen_anilist(&mut self, id: i64) {
        self.seen_anilist.insert(id);
    }

    pub fn is_seen_ann(&self, guid: &str) -> bool {
        self.seen_ann.contains(guid)
    }

    pub fn is_seen_anilist(&self, id: i64) -> bool {
        self.seen_anilist.contains(&id)
    }

    pub fn increment_articles_sent(&mut self) {
        self.stats.total_articles_sent += 1;
        self.update_last_check();
    }

    pub fn increment_episodes_sent(&mut self) {
        self.stats.total_episodes_sent += 1;
        self.update_last_check();
    }

    pub fn increment_errors(&mut self) {
        self.stats.total_errors += 1;
    }

    pub fn update_last_check(&mut self) {
        self.stats.last_check = Some(chrono::Utc::now().to_rfc3339());
    }

    #[allow(dead_code)]
    pub fn clear_seen_data(&mut self) {
        self.seen_ann.clear();
        self.seen_anilist.clear();
        info!("🔄 Cleared all seen data");
    }

    #[allow(dead_code)]
    pub fn reset_stats(&mut self) {
        self.stats = Stats::default();
        info!("📊 Reset statistics");
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

#[allow(dead_code)]
pub fn create_shared_state(state: AppState) -> SharedAppState {
    Arc::new(Mutex::new(state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_operations() {
        let mut state = AppState::default();
        state.add_seen_ann("test_guid".to_string());
        assert!(state.is_seen_ann("test_guid"));

        state.add_seen_anilist(123);
        assert!(state.is_seen_anilist(123));

        state.increment_articles_sent();
        assert_eq!(state.stats.total_articles_sent, 1);
    }
}
