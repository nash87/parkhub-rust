//! Demo mode — time-limited demo with collaborative voting reset.
//!
//! Activated by the `DEMO_MODE=true` environment variable (read once at startup).
//! State is kept in-memory — ephemeral by design for free-tier hosting.
//! Supports actual database reset and scheduled auto-reset every 6 hours.

use axum::{extract::ConnectInfo, http::StatusCode, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
        let cutoff = Instant::now() - Duration::from_secs(VIEWER_TIMEOUT_SECS);
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
        self.next_scheduled_reset =
            Some(now + chrono::Duration::hours(AUTO_RESET_INTERVAL_HOURS));
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
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
    if !s.enabled {
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

    Json(resp).into_response()
}

/// POST /api/v1/demo/vote
pub async fn demo_vote(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    axum::extract::Extension(state): axum::extract::Extension<SharedDemoState>,
) -> impl IntoResponse {
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
    if !s.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Demo mode is not enabled"})),
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

    if s.votes.len() >= VOTE_THRESHOLD {
        s.reset();
        return Json(VoteResponse {
            message: "Demo reset! Page will reload.".into(),
            votes: 0,
            threshold: VOTE_THRESHOLD,
            reset: true,
        })
        .into_response();
    }

    Json(VoteResponse {
        message: "Vote recorded".into(),
        votes: s.votes.len(),
        threshold: VOTE_THRESHOLD,
        reset: false,
    })
    .into_response()
}

/// POST /api/v1/demo/reset — solo reset (only when viewers <= 1)
pub async fn demo_reset(
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    axum::extract::Extension(state): axum::extract::Extension<SharedDemoState>,
) -> impl IntoResponse {
    let mut s = state.lock().unwrap_or_else(|e| e.into_inner());
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

    s.reset();
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
    let s = state.lock().unwrap_or_else(|e| e.into_inner());
    Json(DemoConfigResponse {
        demo_mode: s.enabled,
    })
}
