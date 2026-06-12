use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Maximum number of seen items to retain per source.
/// Prevents unbounded memory/disk growth over months of operation.
const MAX_SEEN_ANN: usize = 1000;
const MAX_SEEN_ANILIST: usize = 500;

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
    /// Load state from YAML file, or create a new empty state (async)
    pub async fn load() -> Self {
        match tokio::fs::read_to_string(STATE_FILE).await {
            Ok(content) => match serde_yml::from_str::<AppState>(&content) {
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

    /// Save state to YAML file (async, non-blocking)
    pub async fn save(&self) -> Result<()> {
        let yaml = serde_yml::to_string(self).context("Failed to serialize state to YAML")?;
        tokio::fs::write(STATE_FILE, yaml)
            .await
            .context(format!("Failed to write {}", STATE_FILE))?;
        debug!("💾 State saved to {}", STATE_FILE);
        Ok(())
    }

    pub fn add_seen_ann(&mut self, guid: String) {
        self.seen_ann.insert(guid);
        self.prune_seen_ann();
    }

    pub fn add_seen_anilist(&mut self, id: i64) {
        self.seen_anilist.insert(id);
        self.prune_seen_anilist();
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

    /// Prune the ANN seen set to prevent unbounded growth.
    /// When the set exceeds the max, we keep only the most recent half.
    /// Since HashSet is unordered, we just drain to the limit.
    fn prune_seen_ann(&mut self) {
        if self.seen_ann.len() > MAX_SEEN_ANN {
            let to_remove = self.seen_ann.len() - (MAX_SEEN_ANN / 2);
            let keys: Vec<String> = self.seen_ann.iter().take(to_remove).cloned().collect();
            for key in keys {
                self.seen_ann.remove(&key);
            }
            debug!(
                "🧹 Pruned ANN seen set from {} to {} entries",
                MAX_SEEN_ANN,
                self.seen_ann.len()
            );
        }
    }

    /// Prune the AniList seen set to prevent unbounded growth.
    fn prune_seen_anilist(&mut self) {
        if self.seen_anilist.len() > MAX_SEEN_ANILIST {
            let to_remove = self.seen_anilist.len() - (MAX_SEEN_ANILIST / 2);
            let keys: Vec<i64> = self.seen_anilist.iter().take(to_remove).cloned().collect();
            for key in keys {
                self.seen_anilist.remove(&key);
            }
            debug!(
                "🧹 Pruned AniList seen set from {} to {} entries",
                MAX_SEEN_ANILIST,
                self.seen_anilist.len()
            );
        }
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

#[cfg(test)]
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

    #[test]
    fn test_state_default() {
        let state = AppState::default();
        assert!(!state.initialized);
        assert!(state.seen_ann.is_empty());
        assert!(state.seen_anilist.is_empty());
        assert_eq!(state.stats.total_articles_sent, 0);
        assert_eq!(state.stats.total_episodes_sent, 0);
        assert_eq!(state.stats.total_errors, 0);
        assert!(state.stats.last_check.is_none());
    }

    #[test]
    fn test_increment_errors() {
        let mut state = AppState::default();
        state.increment_errors();
        state.increment_errors();
        assert_eq!(state.stats.total_errors, 2);
    }

    #[test]
    fn test_prune_seen_ann() {
        let mut state = AppState::default();
        for i in 0..1100 {
            state.add_seen_ann(format!("guid_{}", i));
        }
        // After pruning, should be around MAX_SEEN_ANN / 2
        assert!(state.seen_ann.len() <= MAX_SEEN_ANN);
    }

    #[test]
    fn test_prune_seen_anilist() {
        let mut state = AppState::default();
        for i in 0..600 {
            state.add_seen_anilist(i);
        }
        assert!(state.seen_anilist.len() <= MAX_SEEN_ANILIST);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut state = AppState::default();
        state.add_seen_ann("test".to_string());
        state.add_seen_anilist(42);
        state.initialized = true;
        state.increment_articles_sent();

        let yaml = serde_yml::to_string(&state).unwrap();
        let deserialized: AppState = serde_yml::from_str(&yaml).unwrap();

        assert!(deserialized.is_seen_ann("test"));
        assert!(deserialized.is_seen_anilist(42));
        assert!(deserialized.initialized);
        assert_eq!(deserialized.stats.total_articles_sent, 1);
    }
}
