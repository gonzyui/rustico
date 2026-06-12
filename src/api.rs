use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use serde_json::json;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::config::Config;
use crate::state::SharedAppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
    uptime_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<Stats>,
}

#[derive(Serialize)]
pub struct Stats {
    articles_sent: u64,
    episodes_sent: u64,
    errors: u64,
    seen_articles: usize,
    seen_episodes: usize,
    last_check: Option<String>,
}

#[derive(Clone)]
pub struct ApiState {
    pub shared_state: SharedAppState,
    pub start_time: std::time::Instant,
}

async fn health_handler(State(api_state): State<ApiState>) -> impl IntoResponse {
    let guard = api_state.shared_state.lock().await;
    let uptime = api_state.start_time.elapsed().as_secs();

    let response = HealthResponse {
        status: "🟢 healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        stats: Some(Stats {
            articles_sent: guard.stats.total_articles_sent,
            episodes_sent: guard.stats.total_episodes_sent,
            errors: guard.stats.total_errors,
            seen_articles: guard.seen_ann.len(),
            seen_episodes: guard.seen_anilist.len(),
            last_check: guard.stats.last_check.clone(),
        }),
    };

    (StatusCode::OK, Json(response))
}

async fn metrics_handler(State(api_state): State<ApiState>) -> impl IntoResponse {
    let guard = api_state.shared_state.lock().await;

    let metrics = json!({
        "metrics": {
            "articles_sent": guard.stats.total_articles_sent,
            "episodes_sent": guard.stats.total_episodes_sent,
            "errors": guard.stats.total_errors,
            "seen_articles": guard.seen_ann.len(),
            "seen_episodes": guard.seen_anilist.len(),
            "last_check": guard.stats.last_check.clone(),
        }
    });

    (StatusCode::OK, Json(metrics))
}

async fn stats_handler(State(api_state): State<ApiState>) -> impl IntoResponse {
    let guard = api_state.shared_state.lock().await;

    let stats = json!({
        "initialized": guard.initialized,
        "stats": {
            "articles_sent": guard.stats.total_articles_sent,
            "episodes_sent": guard.stats.total_episodes_sent,
            "errors": guard.stats.total_errors,
        },
        "cache": {
            "seen_articles": guard.seen_ann.len(),
            "seen_episodes": guard.seen_anilist.len(),
        }
    });

    (StatusCode::OK, Json(stats))
}

pub async fn start_health_api(
    config: &Config,
    shared_state: SharedAppState,
    shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    if !config.api.enabled {
        info!("ℹ️ Health API is disabled");
        return Ok(());
    }

    // Concurrency limit: max 50 concurrent requests to prevent abuse
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/stats", get(stats_handler))
        .layer(ConcurrencyLimitLayer::new(50))
        .layer(TraceLayer::new_for_http())
        .with_state(ApiState {
            shared_state,
            start_time: std::time::Instant::now(),
        });

    let addr = format!("{}:{}", config.api.host, config.api.port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind API server to {}: {}", addr, e))?;

    info!("🌐 Health API listening on http://{}", addr);
    info!("   GET http://{}/health - Health check", addr);
    info!("   GET http://{}/metrics - Metrics", addr);
    info!("   GET http://{}/stats - Statistics", addr);

    // Graceful shutdown: wait for the shutdown signal
    let mut shutdown = shutdown_rx;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown.changed().await;
        })
        .await
        .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;

    info!("🛑 Health API shut down gracefully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{create_shared_state, AppState};

    #[test]
    fn test_api_state_creation() {
        let api_state = ApiState {
            shared_state: create_shared_state(AppState::default()),
            start_time: std::time::Instant::now(),
        };
        assert!(api_state.start_time.elapsed().as_secs() < 1);
    }

    #[tokio::test]
    async fn test_health_handler_response() {
        let state = create_shared_state(AppState::default());
        let api_state = ApiState {
            shared_state: state,
            start_time: std::time::Instant::now(),
        };

        let response = health_handler(State(api_state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_handler_response() {
        let state = create_shared_state(AppState::default());
        let api_state = ApiState {
            shared_state: state,
            start_time: std::time::Instant::now(),
        };

        let response = metrics_handler(State(api_state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_stats_handler_response() {
        let mut app_state = AppState::default();
        app_state.increment_articles_sent();
        app_state.increment_episodes_sent();
        let state = create_shared_state(app_state);
        let api_state = ApiState {
            shared_state: state,
            start_time: std::time::Instant::now(),
        };

        let response = stats_handler(State(api_state)).await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
