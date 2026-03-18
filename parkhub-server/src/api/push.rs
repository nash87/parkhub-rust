//! Web Push notification handlers: subscribe, unsubscribe, VAPID key.

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::db::PushSubscription;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub endpoint: String,
    pub keys: PushKeys,
}

#[derive(Debug, Deserialize)]
pub struct PushKeys {
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionResponse {
    pub id: String,
    pub endpoint: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct VapidKeyResponse {
    pub public_key: String,
}

impl From<&PushSubscription> for SubscriptionResponse {
    fn from(s: &PushSubscription) -> Self {
        Self {
            id: s.id.to_string(),
            endpoint: s.endpoint.clone(),
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/push/vapid-key` — return the public VAPID key (no auth).
///
/// Reads `VAPID_PUBLIC_KEY` from the environment. Returns 404 if not configured.
pub(crate) async fn get_vapid_key() -> (StatusCode, Json<ApiResponse<VapidKeyResponse>>) {
    match std::env::var("VAPID_PUBLIC_KEY") {
        Ok(key) if !key.is_empty() => (
            StatusCode::OK,
            Json(ApiResponse::success(VapidKeyResponse { public_key: key })),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NOT_CONFIGURED",
                "VAPID keys are not configured",
            )),
        ),
    }
}

/// `POST /api/v1/push/subscribe` — register a push subscription for the current user.
pub(crate) async fn subscribe(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<SubscribeRequest>,
) -> (StatusCode, Json<ApiResponse<SubscriptionResponse>>) {
    // Basic validation
    if req.endpoint.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "endpoint must not be empty",
            )),
        );
    }
    if req.keys.p256dh.is_empty() || req.keys.auth.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "keys.p256dh and keys.auth must not be empty",
            )),
        );
    }

    let sub = PushSubscription {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        endpoint: req.endpoint,
        p256dh: req.keys.p256dh,
        auth: req.keys.auth,
        created_at: Utc::now(),
    };

    let state_guard = state.read().await;
    match state_guard.db.save_push_subscription(&sub).await {
        Ok(()) => {
            let resp = SubscriptionResponse::from(&sub);
            (StatusCode::CREATED, Json(ApiResponse::success(resp)))
        }
        Err(e) => {
            tracing::error!("Failed to save push subscription: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

/// `DELETE /api/v1/push/unsubscribe` — remove all push subscriptions for the current user.
pub(crate) async fn unsubscribe(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    match state_guard
        .db
        .delete_push_subscriptions_by_user(&auth_user.user_id)
        .await
    {
        Ok(count) => {
            tracing::info!(
                "Deleted {} push subscription(s) for user {}",
                count,
                auth_user.user_id
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to delete push subscriptions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Push delivery (placeholder)
// ─────────────────────────────────────────────────────────────────────────────

/// Send a push notification to all subscriptions for the given user.
///
/// This is a placeholder that logs the attempt. Replace with the `web-push`
/// crate for real Web Push Protocol delivery.
#[allow(dead_code)]
pub async fn send_push_notification(
    db: &crate::db::Database,
    user_id: &Uuid,
    title: &str,
    body: &str,
) {
    match db.get_push_subscriptions_by_user(user_id).await {
        Ok(subs) if subs.is_empty() => {
            tracing::debug!("No push subscriptions for user {}", user_id);
        }
        Ok(subs) => {
            for sub in &subs {
                tracing::info!(
                    "Would send push to user {} endpoint={} title={:?} body={:?}",
                    user_id,
                    sub.endpoint,
                    title,
                    body,
                );
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to list push subscriptions for user {}: {}",
                user_id,
                e
            );
        }
    }
}
