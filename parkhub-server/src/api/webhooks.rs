//! Webhook handlers: CRUD operations and event dispatch.
//!
//! All endpoints are admin-only. Webhook URLs must be HTTPS and pass
//! SSRF validation (no private IPs, no localhost in production).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::net::IpAddr;
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use crate::db::Webhook;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub events: Vec<String>,
    #[serde(default = "default_true")]
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWebhookRequest {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub active: Option<bool>,
    /// If true, regenerate the secret
    #[serde(default)]
    pub regenerate_secret: bool,
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

impl From<&Webhook> for WebhookResponse {
    fn from(w: &Webhook) -> Self {
        Self {
            id: w.id.to_string(),
            url: w.url.clone(),
            secret: w.secret.clone(),
            events: w.events.clone(),
            active: w.active,
            created_at: w.created_at.to_rfc3339(),
            updated_at: w.updated_at.to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SSRF protection
// ─────────────────────────────────────────────────────────────────────────────

/// Known event types for validation
const VALID_EVENTS: &[&str] = &[
    "booking.created",
    "booking.cancelled",
    "booking.updated",
    "user.created",
    "user.deleted",
    "lot.created",
    "lot.updated",
    "lot.deleted",
    "test",
];

/// Validate a webhook URL for SSRF safety.
/// Returns an error message string if invalid, None if OK.
fn validate_webhook_url(url: &str) -> Option<&'static str> {
    let parsed = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return Some("Invalid URL format"),
    };

    // Must be HTTPS (allow HTTP only for localhost in dev builds)
    if parsed.scheme() != "https" {
        #[cfg(debug_assertions)]
        {
            let host = parsed.host_str().unwrap_or("");
            if parsed.scheme() != "http" || (host != "localhost" && host != "127.0.0.1") {
                return Some("URL must use HTTPS");
            }
        }
        #[cfg(not(debug_assertions))]
        {
            return Some("URL must use HTTPS");
        }
    }

    let host = match parsed.host_str() {
        Some(h) => h,
        None => return Some("URL must have a host"),
    };

    // Block localhost and related
    let lower = host.to_lowercase();
    if lower == "localhost" || lower == "0.0.0.0" || lower == "[::1]" || lower == "[::]" {
        #[cfg(not(debug_assertions))]
        {
            return Some("URL must not target localhost");
        }
    }

    // Try to parse as IP for private range checks
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(&ip) {
            return Some("URL must not target private/reserved IP ranges");
        }
    }

    None
}

fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            // 10.0.0.0/8
            octets[0] == 10
            // 172.16.0.0/12
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            // 192.168.0.0/16
            || (octets[0] == 192 && octets[1] == 168)
            // 127.0.0.0/8 (loopback)
            || octets[0] == 127
            // 169.254.0.0/16 (link-local)
            || (octets[0] == 169 && octets[1] == 254)
            // 0.0.0.0
            || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unspecified()
        }
    }
}

/// Generate a cryptographically random secret for HMAC signing.
fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    format!("whsec_{}", hex::encode(bytes))
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/webhooks` — list all webhooks (admin only)
pub(crate) async fn list_webhooks(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<WebhookResponse>>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    match state_guard.db.list_webhooks().await {
        Ok(webhooks) => {
            let responses: Vec<WebhookResponse> = webhooks.iter().map(WebhookResponse::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("Failed to list webhooks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list webhooks")),
            )
        }
    }
}

/// `POST /api/v1/webhooks` — create a new webhook (admin only)
pub(crate) async fn create_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateWebhookRequest>,
) -> (StatusCode, Json<ApiResponse<WebhookResponse>>) {
    let state_guard = state.write().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    // Validate URL
    if let Some(err) = validate_webhook_url(&req.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("VALIDATION_ERROR", err)),
        );
    }

    // Validate events
    for event in &req.events {
        if !VALID_EVENTS.contains(&event.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "Invalid event type. Valid: booking.created, booking.cancelled, booking.updated, user.created, user.deleted, lot.created, lot.updated, lot.deleted, test",
                )),
            );
        }
    }

    if req.events.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "At least one event type is required",
            )),
        );
    }

    let now = Utc::now();
    let webhook = Webhook {
        id: Uuid::new_v4(),
        url: req.url,
        secret: generate_secret(),
        events: req.events,
        active: req.active,
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = state_guard.db.save_webhook(&webhook).await {
        tracing::error!("Failed to save webhook: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create webhook")),
        );
    }

    tracing::info!("Created webhook {} -> {}", webhook.id, webhook.url);

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(WebhookResponse::from(&webhook))),
    )
}

/// `PUT /api/v1/webhooks/{id}` — update a webhook (admin only)
pub(crate) async fn update_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWebhookRequest>,
) -> (StatusCode, Json<ApiResponse<WebhookResponse>>) {
    let state_guard = state.write().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    // Fetch existing webhook
    let mut webhook = match state_guard.db.get_webhook(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Apply updates
    if let Some(ref url) = req.url {
        if let Some(err) = validate_webhook_url(url) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", err)),
            );
        }
        webhook.url = url.clone();
    }

    if let Some(ref events) = req.events {
        for event in events {
            if !VALID_EVENTS.contains(&event.as_str()) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "VALIDATION_ERROR",
                        "Invalid event type",
                    )),
                );
            }
        }
        if events.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "VALIDATION_ERROR",
                    "At least one event type is required",
                )),
            );
        }
        webhook.events = events.clone();
    }

    if let Some(active) = req.active {
        webhook.active = active;
    }

    if req.regenerate_secret {
        webhook.secret = generate_secret();
    }

    webhook.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_webhook(&webhook).await {
        tracing::error!("Failed to update webhook: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update webhook",
            )),
        );
    }

    tracing::info!("Updated webhook {}", webhook.id);

    (
        StatusCode::OK,
        Json(ApiResponse::success(WebhookResponse::from(&webhook))),
    )
}

/// `DELETE /api/v1/webhooks/{id}` — delete a webhook (admin only)
pub(crate) async fn delete_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    match state_guard.db.delete_webhook(&id).await {
        Ok(true) => {
            tracing::info!("Deleted webhook: {}", id);
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete webhook: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete webhook",
                )),
            )
        }
    }
}

/// `POST /api/v1/webhooks/{id}/test` — send a test event (admin only)
pub(crate) async fn test_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    // Admin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    let webhook = match state_guard.db.get_webhook(&id).await {
        Ok(Some(w)) => w,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let payload = serde_json::json!({
        "event": "test",
        "timestamp": Utc::now().to_rfc3339(),
        "data": {
            "message": "This is a test webhook delivery from ParkHub."
        }
    });

    let body = serde_json::to_string(&payload).unwrap_or_default();
    let signature = compute_signature(&webhook.secret, &body);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    match client
        .post(&webhook.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", &signature)
        .body(body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let result = serde_json::json!({
                "delivered": true,
                "status_code": status,
            });
            tracing::info!(
                "Test webhook {} delivered to {} — HTTP {}",
                id,
                webhook.url,
                status
            );
            (StatusCode::OK, Json(ApiResponse::success(result)))
        }
        Err(e) => {
            let result = serde_json::json!({
                "delivered": false,
                "error": e.to_string(),
            });
            tracing::warn!("Test webhook {} delivery failed: {}", id, e);
            (StatusCode::OK, Json(ApiResponse::success(result)))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Event dispatch
// ─────────────────────────────────────────────────────────────────────────────

/// Compute HMAC-SHA256 signature of the body using the webhook secret.
fn compute_signature(secret: &str, body: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(body.as_bytes());
    let result = mac.finalize();
    format!("sha256={}", hex::encode(result.into_bytes()))
}

/// Dispatch a webhook event to all matching active webhooks.
///
/// Non-blocking: reads webhooks from DB, spawns a task per match, returns immediately.
/// Delivery failures are logged but not retried.
///
/// Call from any async handler that has access to `SharedState`.
#[allow(dead_code)]
pub async fn dispatch_webhook_event(
    state: &SharedState,
    event_type: &str,
    payload: serde_json::Value,
) {
    let state_guard = state.read().await;
    let webhooks = match state_guard.db.list_webhooks().await {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to list webhooks for dispatch: {}", e);
            return;
        }
    };
    // Drop the lock before spawning tasks
    drop(state_guard);

    let matching: Vec<Webhook> = webhooks
        .into_iter()
        .filter(|w| w.active && w.events.iter().any(|e| e == event_type || e == "*"))
        .collect();

    if matching.is_empty() {
        return;
    }

    let event_type = event_type.to_string();
    let body = serde_json::to_string(&serde_json::json!({
        "event": event_type,
        "timestamp": Utc::now().to_rfc3339(),
        "data": payload,
    }))
    .unwrap_or_default();

    for webhook in matching {
        let body = body.clone();
        let url = webhook.url.clone();
        let secret = webhook.secret.clone();
        let event = event_type.clone();

        tokio::spawn(async move {
            let signature = compute_signature(&secret, &body);
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();

            match client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Signature", &signature)
                .body(body)
                .send()
                .await
            {
                Ok(resp) => {
                    tracing::info!(
                        "Webhook {} delivered event '{}' to {} — HTTP {}",
                        webhook.id,
                        event,
                        url,
                        resp.status()
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Webhook {} failed to deliver '{}' to {}: {}",
                        webhook.id,
                        event,
                        url,
                        e
                    );
                }
            }
        });
    }
}
