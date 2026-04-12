//! Rate limiting dashboard: admin endpoints for monitoring rate limit statistics.

use axum::{Extension, Json, extract::State};
use chrono::{TimeDelta, Utc};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::ApiResponse;

use super::{AuthUser, check_admin};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// A single rate limit group's status
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitGroupStatus {
    pub group: String,
    pub limit_per_minute: u32,
    pub description: String,
    pub current_count: u32,
    pub reset_seconds: u64,
    pub blocked_last_hour: u32,
}

/// Response for rate limit stats
#[derive(Debug, Serialize)]
pub struct RateLimitStats {
    pub groups: Vec<RateLimitGroupStatus>,
    pub total_blocked_last_hour: u32,
}

/// Hourly bin for blocked-request history
#[derive(Debug, Clone, Serialize)]
pub struct BlockedRequestBin {
    pub hour: String,
    pub count: u32,
}

/// Response for blocked request history
#[derive(Debug, Serialize)]
pub struct BlockedRequestHistory {
    pub bins: Vec<BlockedRequestBin>,
}

/// `GET /api/v1/admin/rate-limits` — current rate limit stats per endpoint group
#[utoipa::path(get, path = "/api/v1/admin/rate-limits", tag = "Admin",
    summary = "Rate limit stats",
    description = "Returns current rate limit statistics per endpoint group.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Rate limit stats"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_rate_limit_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<ApiResponse<RateLimitStats>>, (axum::http::StatusCode, &'static str)> {
    let state_guard = state.read().await;
    check_admin(&state_guard, &auth_user).await?;

    // Read blocked counts from settings (incremented by rate limit middleware)
    let blocked_auth = read_blocked_count(&state_guard, "rate_blocked:auth").await;
    let blocked_api = read_blocked_count(&state_guard, "rate_blocked:api").await;
    let blocked_public = read_blocked_count(&state_guard, "rate_blocked:public").await;
    let blocked_webhook = read_blocked_count(&state_guard, "rate_blocked:webhook").await;

    let groups = vec![
        RateLimitGroupStatus {
            group: "auth".to_string(),
            limit_per_minute: 5,
            description: "Authentication (login, register, password reset)".to_string(),
            current_count: 0, // Governor doesn't expose current count; show 0
            reset_seconds: 60,
            blocked_last_hour: blocked_auth,
        },
        RateLimitGroupStatus {
            group: "api".to_string(),
            limit_per_minute: 100,
            description: "General API requests".to_string(),
            current_count: 0,
            reset_seconds: 60,
            blocked_last_hour: blocked_api,
        },
        RateLimitGroupStatus {
            group: "public".to_string(),
            limit_per_minute: 30,
            description: "Public endpoints (lobby display, QR, OAuth)".to_string(),
            current_count: 0,
            reset_seconds: 60,
            blocked_last_hour: blocked_public,
        },
        RateLimitGroupStatus {
            group: "webhook".to_string(),
            limit_per_minute: 50,
            description: "Webhook endpoints (Stripe, integrations)".to_string(),
            current_count: 0,
            reset_seconds: 60,
            blocked_last_hour: blocked_webhook,
        },
    ];

    let total_blocked = blocked_auth + blocked_api + blocked_public + blocked_webhook;

    Ok(Json(ApiResponse::success(RateLimitStats {
        groups,
        total_blocked_last_hour: total_blocked,
    })))
}

/// `GET /api/v1/admin/rate-limits/history` — blocked requests over last 24h
#[utoipa::path(get, path = "/api/v1/admin/rate-limits/history", tag = "Admin",
    summary = "Blocked request history",
    description = "Returns blocked requests over the last 24 hours in hourly bins.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Blocked request history"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_rate_limit_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<ApiResponse<BlockedRequestHistory>>, (axum::http::StatusCode, &'static str)> {
    let state_guard = state.read().await;
    check_admin(&state_guard, &auth_user).await?;

    let now = Utc::now();
    let mut bins = Vec::with_capacity(24);

    for i in (0..24).rev() {
        let hour = now - TimeDelta::hours(i);
        let hour_key = hour.format("%Y-%m-%dT%H").to_string();
        let setting_key = format!("rate_blocked_hourly:{hour_key}");
        let count = read_blocked_count(&state_guard, &setting_key).await;
        bins.push(BlockedRequestBin {
            hour: format!("{hour_key}:00"),
            count,
        });
    }

    Ok(Json(ApiResponse::success(BlockedRequestHistory { bins })))
}

/// Read a blocked count from the settings DB. Returns 0 if not found.
async fn read_blocked_count(state: &AppState, key: &str) -> u32 {
    state
        .db
        .get_setting(key)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_group_serialize() {
        let group = RateLimitGroupStatus {
            group: "auth".to_string(),
            limit_per_minute: 5,
            description: "Authentication".to_string(),
            current_count: 3,
            reset_seconds: 42,
            blocked_last_hour: 7,
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("\"group\":\"auth\""));
        assert!(json.contains("\"limit_per_minute\":5"));
        assert!(json.contains("\"blocked_last_hour\":7"));
    }

    #[test]
    fn test_rate_limit_stats_serialize() {
        let stats = RateLimitStats {
            groups: vec![RateLimitGroupStatus {
                group: "api".to_string(),
                limit_per_minute: 100,
                description: "General API".to_string(),
                current_count: 0,
                reset_seconds: 60,
                blocked_last_hour: 0,
            }],
            total_blocked_last_hour: 0,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_blocked_last_hour\":0"));
        assert!(json.contains("\"groups\":["));
    }

    #[test]
    fn test_blocked_request_bin_serialize() {
        let bin = BlockedRequestBin {
            hour: "2026-03-22T14:00".to_string(),
            count: 42,
        };
        let json = serde_json::to_string(&bin).unwrap();
        assert!(json.contains("\"hour\":\"2026-03-22T14:00\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_blocked_request_history_24_bins() {
        let now = Utc::now();
        let mut bins = Vec::with_capacity(24);
        for i in (0..24).rev() {
            let hour = now - TimeDelta::hours(i);
            bins.push(BlockedRequestBin {
                hour: hour.format("%Y-%m-%dT%H:00").to_string(),
                count: 0,
            });
        }
        let history = BlockedRequestHistory { bins };
        assert_eq!(history.bins.len(), 24);
        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("\"bins\":["));
    }
}
