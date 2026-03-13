//! Demo mode — time-limited demo with collaborative voting reset.
//!
//! Activated by the `DEMO_MODE=true` environment variable (read once at startup).
//! State is kept in-memory — ephemeral by design for free-tier hosting.

use axum::{
    extract::ConnectInfo,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TIMER_DURATION_SECS: u64 = 1800; // 30 minutes
const VOTE_THRESHOLD: usize = 3;
const VIEWER_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// In-memory demo state (cheap, ephemeral — resets on restart).
#[derive(Debug)]
pub struct DemoState {
    pub enabled: bool,
    started_at: Instant,
    votes: HashMap<String, Instant>,
    viewers: HashMap<String, Instant>,
}

impl DemoState {
    pub fn new() -> Self {
        let enabled = std::env::var("DEMO_MODE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        Self {
            enabled,
            started_at: Instant::now(),
            votes: HashMap::new(),
            viewers: HashMap::new(),
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

    fn reset(&mut self) {
        self.started_at = Instant::now();
        self.votes.clear();
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

/// GET /api/v1/demo/config
pub async fn demo_config(
    axum::extract::Extension(state): axum::extract::Extension<SharedDemoState>,
) -> Json<DemoConfigResponse> {
    let s = state.lock().unwrap_or_else(|e| e.into_inner());
    Json(DemoConfigResponse {
        demo_mode: s.enabled,
    })
}
