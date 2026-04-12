//! Demo mode — time-limited demo with collaborative voting reset.
//!
//! Activated by the `DEMO_MODE=true` environment variable (read once at startup).
//! State is kept in-memory — ephemeral by design for free-tier hosting.
//! Supports actual database reset and scheduled auto-reset every 6 hours.

use axum::{Extension, Json, extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::info;

use crate::AppState;

type SharedAppState = Arc<RwLock<AppState>>;

const TIMER_DURATION_SECS: u64 = 1800; // 30 minutes
const VOTE_THRESHOLD: usize = 3;
const VIEWER_TIMEOUT_SECS: u64 = 300; // 5 minutes
pub const AUTO_RESET_INTERVAL_HOURS: i64 = 6;

/// In-memory demo state (cheap, ephemeral — resets on restart).
#[derive(Debug)]
pub struct DemoState {
    pub enabled: bool,
    started_at: Instant,
    votes: HashMap<String, Instant>,
    viewers: HashMap<String, Instant>,
    /// When the last full data reset occurred (None = never / fresh start)
    pub last_reset_at: Option<DateTime<Utc>>,
    /// When the next scheduled auto-reset will fire
    pub next_scheduled_reset: Option<DateTime<Utc>>,
    /// True while a reset is in progress (prevents concurrent resets)
    pub reset_in_progress: bool,
}

impl DemoState {
    pub fn new() -> Self {
        let enabled = std::env::var("DEMO_MODE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let now = Utc::now();
        Self {
            enabled,
            started_at: Instant::now(),
            votes: HashMap::new(),
            viewers: HashMap::new(),
            last_reset_at: if enabled { Some(now) } else { None },
            next_scheduled_reset: if enabled {
                Some(now + chrono::Duration::hours(AUTO_RESET_INTERVAL_HOURS))
            } else {
                None
            },
            reset_in_progress: false,
        }
    }

    fn remaining_secs(&self) -> u64 {
        let elapsed = self.started_at.elapsed().as_secs();
        TIMER_DURATION_SECS.saturating_sub(elapsed)
    }

    fn prune_viewers(&mut self) {
        let cutoff = Instant::now()
            .checked_sub(Duration::from_secs(VIEWER_TIMEOUT_SECS))
            .unwrap();
        self.viewers.retain(|_, ts| *ts > cutoff);
    }

    /// Reset the in-memory timer and vote state.
    /// Call `mark_reset_complete()` after the full DB reset finishes.
    pub fn reset(&mut self) {
        self.started_at = Instant::now();
        self.votes.clear();
    }

    /// Mark a full data reset as complete — updates timestamps.
    pub fn mark_reset_complete(&mut self) {
        let now = Utc::now();
        self.last_reset_at = Some(now);
        self.next_scheduled_reset = Some(now + chrono::Duration::hours(AUTO_RESET_INTERVAL_HOURS));
        self.reset_in_progress = false;
    }
}

pub type SharedDemoState = Arc<Mutex<DemoState>>;

pub fn new_demo_state() -> SharedDemoState {
    Arc::new(Mutex::new(DemoState::new()))
}

#[derive(Serialize)]
struct DemoStatusResponse {
    enabled: bool,
    timer: TimerInfo,
    votes: VoteInfo,
    viewers: usize,
    /// ISO 8601 timestamp of last reset (null if never)
    last_reset_at: Option<String>,
    /// ISO 8601 timestamp of next scheduled auto-reset
    next_scheduled_reset: Option<String>,
    /// True while a reset is running
    reset_in_progress: bool,
}

#[derive(Serialize)]
struct TimerInfo {
    remaining: u64,
    duration: u64,
}

#[derive(Serialize)]
struct VoteInfo {
    current: usize,
    threshold: usize,
    has_voted: bool,
}

#[derive(Serialize)]
struct VoteResponse {
    message: String,
    votes: usize,
    threshold: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    reset: bool,
}

#[derive(Serialize)]
pub struct DemoConfigResponse {
    demo_mode: bool,
}

fn client_ip(addr: &SocketAddr) -> String {
    addr.ip().to_string()
}

/// GET /api/v1/demo/status
pub async fn demo_status(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::Extension(state): axum::extract::Extension<SharedDemoState>,
) -> impl IntoResponse {
    let mut s = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !s.enabled {
        drop(s);
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Demo mode is not enabled"})),
        )
            .into_response();
    }

    let ip = client_ip(&addr);
    s.viewers.insert(ip.clone(), Instant::now());
    s.prune_viewers();

    let has_voted = s.votes.contains_key(&ip);
    let resp = DemoStatusResponse {
        enabled: true,
        timer: TimerInfo {
            remaining: s.remaining_secs(),
            duration: TIMER_DURATION_SECS,
        },
        votes: VoteInfo {
            current: s.votes.len(),
            threshold: VOTE_THRESHOLD,
            has_voted,
        },
        viewers: s.viewers.len(),
        last_reset_at: s.last_reset_at.map(|t| t.to_rfc3339()),
        next_scheduled_reset: s.next_scheduled_reset.map(|t| t.to_rfc3339()),
        reset_in_progress: s.reset_in_progress,
    };
    drop(s);

    Json(resp).into_response()
}

/// Perform a full demo data reset: clear DB, re-seed admin + sample lot + credits.
/// Always clears `reset_in_progress` on exit, even on partial failure.
async fn perform_db_reset(app_state: &SharedAppState, demo_state: &SharedDemoState) {
    // Mark reset in progress
    if let Ok(mut ds) = demo_state.lock() {
        ds.reset_in_progress = true;
    }

    let result: Result<(), String> = async {
        let state_guard = app_state.write().await;

        // Clear all data
        state_guard
            .db
            .clear_all_data()
            .await
            .map_err(|e| format!("failed to clear data: {e}"))?;

        // Re-create admin user and sample lot
        crate::create_admin_user(&state_guard.db, &state_guard.config)
            .await
            .map_err(|e| format!("failed to create admin: {e}"))?;
        crate::create_sample_parking_lot(&state_guard.db)
            .await
            .map_err(|e| format!("failed to create sample lot: {e}"))?;

        // Re-enable credits
        let _ = state_guard.db.set_setting("credits_enabled", "true").await;
        let _ = state_guard.db.set_setting("credits_per_booking", "1").await;

        drop(state_guard);
        Ok(())
    }
    .await;

    // Always update demo state and clear reset_in_progress
    if let Ok(mut ds) = demo_state.lock() {
        match &result {
            Ok(()) => {
                ds.reset();
                ds.mark_reset_complete();
                info!("Demo data reset complete");
            }
            Err(e) => {
                ds.reset_in_progress = false;
                tracing::error!("Demo reset failed: {}", e);
            }
        }
    }
}

/// POST /api/v1/demo/vote
pub async fn demo_vote(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<SharedDemoState>,
    Extension(app_state): Extension<SharedAppState>,
) -> impl IntoResponse {
    // Check enabled + existing vote + reset_in_progress under short lock
    let should_reset = {
        let mut s = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !s.enabled {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Demo mode is not enabled"})),
            )
                .into_response();
        }

        if s.reset_in_progress {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "A reset is already in progress"})),
            )
                .into_response();
        }

        let ip = client_ip(&addr);
        if s.votes.contains_key(&ip) {
            return Json(VoteResponse {
                message: "Already voted".into(),
                votes: s.votes.len(),
                threshold: VOTE_THRESHOLD,
                reset: false,
            })
            .into_response();
        }

        s.votes.insert(ip, Instant::now());
        s.votes.len() >= VOTE_THRESHOLD
    };

    if should_reset {
        // Full DB reset — runs outside the Mutex lock
        perform_db_reset(&app_state, &state).await;

        return Json(VoteResponse {
            message: "Demo reset! Page will reload.".into(),
            votes: 0,
            threshold: VOTE_THRESHOLD,
            reset: true,
        })
        .into_response();
    }

    let votes = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .votes
        .len();
    Json(VoteResponse {
        message: "Vote recorded".into(),
        votes,
        threshold: VOTE_THRESHOLD,
        reset: false,
    })
    .into_response()
}

/// POST /api/v1/demo/reset — solo reset (only when viewers <= 1)
pub async fn demo_reset(
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    Extension(state): Extension<SharedDemoState>,
    Extension(app_state): Extension<SharedAppState>,
) -> impl IntoResponse {
    // Pre-checks under short lock
    {
        let mut s = state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !s.enabled {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Demo mode is not enabled"})),
            )
                .into_response();
        }

        s.prune_viewers();

        if s.viewers.len() > 1 {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "Solo reset not available with multiple viewers. Use voting instead.",
                    "viewers": s.viewers.len()
                })),
            )
                .into_response();
        }

        if s.reset_in_progress {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "A reset is already in progress"})),
            )
                .into_response();
        }
    } // lock released

    // Full DB reset
    perform_db_reset(&app_state, &state).await;

    Json(VoteResponse {
        message: "Demo reset! Page will reload.".into(),
        votes: 0,
        threshold: VOTE_THRESHOLD,
        reset: true,
    })
    .into_response()
}

/// GET /api/v1/demo/config
pub async fn demo_config(
    axum::extract::Extension(state): axum::extract::Extension<SharedDemoState>,
) -> Json<DemoConfigResponse> {
    let s = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Json(DemoConfigResponse {
        demo_mode: s.enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a DemoState with `enabled = true` without touching the environment.
    fn enabled_demo_state() -> DemoState {
        let now = Utc::now();
        DemoState {
            enabled: true,
            started_at: Instant::now(),
            votes: HashMap::new(),
            viewers: HashMap::new(),
            last_reset_at: Some(now),
            next_scheduled_reset: Some(now + chrono::Duration::hours(AUTO_RESET_INTERVAL_HOURS)),
            reset_in_progress: false,
        }
    }

    #[test]
    #[allow(unsafe_code)]
    fn demo_state_disabled_by_default_when_env_not_set() {
        // Temporarily ensure DEMO_MODE is unset (guard against test-ordering effects).
        let original = std::env::var("DEMO_MODE").ok();
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("DEMO_MODE") };

        let state = DemoState::new();
        assert!(!state.enabled, "DemoState must default to disabled");
        assert!(
            state.last_reset_at.is_none(),
            "last_reset_at must be None when disabled"
        );
        assert!(
            state.next_scheduled_reset.is_none(),
            "next_scheduled_reset must be None when disabled"
        );

        // Restore original value so other tests are unaffected.
        if let Some(val) = original {
            // SAFETY: single-threaded test or pre-spawn context
            unsafe { std::env::set_var("DEMO_MODE", val) };
        }
    }

    #[test]
    fn demo_state_reset_clears_votes_and_restarts_timer() {
        let mut state = enabled_demo_state();
        state.votes.insert("1.2.3.4".to_string(), Instant::now());
        state.votes.insert("5.6.7.8".to_string(), Instant::now());
        assert_eq!(state.votes.len(), 2);

        state.reset();

        assert!(state.votes.is_empty(), "reset() must clear all votes");
        // started_at should be refreshed; remaining time close to full.
        assert!(
            state.remaining_secs() >= TIMER_DURATION_SECS - 1,
            "remaining_secs should be near full after reset"
        );
    }

    #[test]
    fn demo_state_mark_reset_complete_updates_timestamps_and_clears_flag() {
        let mut state = enabled_demo_state();
        state.reset_in_progress = true;

        let before = Utc::now();
        state.mark_reset_complete();
        let after = Utc::now();

        assert!(
            !state.reset_in_progress,
            "reset_in_progress must be cleared"
        );

        let last = state.last_reset_at.expect("last_reset_at must be set");
        assert!(
            last >= before && last <= after,
            "last_reset_at must be current time"
        );

        let next = state
            .next_scheduled_reset
            .expect("next_scheduled_reset must be set");
        let expected_next = last + chrono::Duration::hours(AUTO_RESET_INTERVAL_HOURS);
        // Allow ±2 s for execution time.
        assert!(
            (next - expected_next).num_seconds().abs() <= 2,
            "next_scheduled_reset must be AUTO_RESET_INTERVAL_HOURS after last_reset_at"
        );
    }

    #[test]
    fn demo_state_remaining_secs_starts_near_full_duration() {
        let state = enabled_demo_state();
        let remaining = state.remaining_secs();
        assert!(
            remaining >= TIMER_DURATION_SECS - 1,
            "fresh state must have nearly full remaining time, got {remaining}"
        );
        assert!(
            remaining <= TIMER_DURATION_SECS,
            "remaining must not exceed total timer duration"
        );
    }

    #[test]
    fn demo_state_prune_viewers_keeps_fresh_entries() {
        let mut state = enabled_demo_state();
        // Insert a viewer right now — it must survive pruning.
        state.viewers.insert("10.0.0.1".to_string(), Instant::now());
        assert_eq!(state.viewers.len(), 1);

        state.prune_viewers();

        assert_eq!(state.viewers.len(), 1, "fresh viewer must not be pruned");
    }

    #[test]
    fn demo_state_vote_threshold_and_count() {
        let mut state = enabled_demo_state();

        // Insert votes up to threshold - 1; should_reset logic lives in the HTTP handler,
        // but here we verify that votes.len() tracks correctly.
        for i in 0..VOTE_THRESHOLD {
            state.votes.insert(format!("ip_{i}"), Instant::now());
        }
        assert_eq!(
            state.votes.len(),
            VOTE_THRESHOLD,
            "vote count must match number of inserted votes"
        );

        let should_reset = state.votes.len() >= VOTE_THRESHOLD;
        assert!(should_reset, "should_reset must be true at threshold");
    }
}
