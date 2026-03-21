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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubscribeRequest {
    /// Push service endpoint URL
    pub endpoint: String,
    /// Browser push encryption keys
    pub keys: PushKeys,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PushKeys {
    /// P-256 Diffie-Hellman public key
    pub p256dh: String,
    /// Authentication secret
    pub auth: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SubscriptionResponse {
    /// Subscription ID
    pub id: String,
    /// Push service endpoint URL
    pub endpoint: String,
    /// Creation timestamp (RFC 3339)
    pub created_at: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VapidKeyResponse {
    /// VAPID public key (base64url-encoded)
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
#[utoipa::path(
    get,
    path = "/api/v1/push/vapid-key",
    tag = "Push",
    summary = "Get VAPID public key",
    description = "Returns the server's VAPID public key for Web Push subscription. No auth required.",
    responses(
        (status = 200, description = "VAPID public key"),
        (status = 404, description = "VAPID keys not configured"),
    )
)]
pub async fn get_vapid_key() -> (StatusCode, Json<ApiResponse<VapidKeyResponse>>) {
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
#[utoipa::path(
    post,
    path = "/api/v1/push/subscribe",
    tag = "Push",
    summary = "Subscribe to push notifications",
    description = "Register a Web Push subscription endpoint for the authenticated user.",
    request_body = SubscribeRequest,
    responses(
        (status = 201, description = "Subscription created"),
        (status = 400, description = "Validation error"),
    )
)]
pub async fn subscribe(
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
#[utoipa::path(
    delete,
    path = "/api/v1/push/unsubscribe",
    tag = "Push",
    summary = "Unsubscribe from push notifications",
    description = "Remove all push notification subscriptions for the authenticated user.",
    responses(
        (status = 200, description = "Subscriptions removed"),
    )
)]
pub async fn unsubscribe(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscribe_request_deserialize() {
        let json = r#"{"endpoint":"https://push.example.com/send/abc","keys":{"p256dh":"BNcR...","auth":"tBH..."}}"#;
        let req: SubscribeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.endpoint, "https://push.example.com/send/abc");
        assert_eq!(req.keys.p256dh, "BNcR...");
        assert_eq!(req.keys.auth, "tBH...");
    }

    #[test]
    fn test_subscribe_request_missing_keys() {
        let json = r#"{"endpoint":"https://push.example.com/send/abc"}"#;
        let result: Result<SubscribeRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_subscribe_request_missing_endpoint() {
        let json = r#"{"keys":{"p256dh":"BNcR...","auth":"tBH..."}}"#;
        let result: Result<SubscribeRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_keys_missing_auth() {
        let json = r#"{"endpoint":"https://example.com","keys":{"p256dh":"BNcR..."}}"#;
        let result: Result<SubscribeRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_vapid_key_response_serialize() {
        let resp = VapidKeyResponse {
            public_key: "BNcRdreALRFXTkOOUHK1EtK2wtaz5Ry4YfYCA_0QTpQtUbVlUls0VJXg7A8u-Ts1XbjhazAkj7I99e8p8ljwlQA".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("public_key"));
        assert!(json.contains("BNcR"));
    }

    #[test]
    fn test_subscription_response_serialize() {
        let resp = SubscriptionResponse {
            id: "test-id".to_string(),
            endpoint: "https://push.example.com".to_string(),
            created_at: "2026-03-20T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["id"], "test-id");
        assert_eq!(value["endpoint"], "https://push.example.com");
        assert_eq!(value["created_at"], "2026-03-20T10:00:00Z");
    }

    #[test]
    fn test_subscription_response_from_push_subscription() {
        let sub = PushSubscription {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            user_id: Uuid::new_v4(),
            endpoint: "https://push.example.com/sub/123".to_string(),
            p256dh: "key1".to_string(),
            auth: "auth1".to_string(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-03-20T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };
        let resp = SubscriptionResponse::from(&sub);
        assert_eq!(resp.id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(resp.endpoint, "https://push.example.com/sub/123");
        assert!(resp.created_at.contains("2026-03-20"));
    }

    #[test]
    fn test_push_subscription_serde_roundtrip() {
        let sub = PushSubscription {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            endpoint: "https://fcm.googleapis.com/fcm/send/abc".to_string(),
            p256dh: "BNcRdreALRFXTkOOUHK1EtK2wtaz5Ry4YfYCA".to_string(),
            auth: "tBHItJI5svbpC7Aq".to_string(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&sub).unwrap();
        let deserialized: PushSubscription = serde_json::from_str(&json).unwrap();
        assert_eq!(sub.id, deserialized.id);
        assert_eq!(sub.endpoint, deserialized.endpoint);
        assert_eq!(sub.p256dh, deserialized.p256dh);
        assert_eq!(sub.auth, deserialized.auth);
    }
}
