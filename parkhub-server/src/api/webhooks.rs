//! Webhook handlers: CRUD operations and event dispatch.
//!
//! All endpoints are admin-only. Webhook URLs must be HTTPS and pass
//! SSRF validation (no private IPs, no localhost in production).

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    /// Target URL for webhook delivery (must be HTTPS)
    pub url: String,
    /// Event types to subscribe to (e.g. "booking.created", "user.deleted")
    pub events: Vec<String>,
    /// Whether the webhook is active (defaults to true)
    #[serde(default = "default_true")]
    pub active: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateWebhookRequest {
    /// New target URL (optional)
    pub url: Option<String>,
    /// New event types (optional)
    pub events: Option<Vec<String>>,
    /// Enable/disable (optional)
    pub active: Option<bool>,
    /// If true, regenerate the HMAC signing secret
    #[serde(default)]
    pub regenerate_secret: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WebhookResponse {
    /// Webhook ID
    pub id: String,
    /// Target URL
    pub url: String,
    /// HMAC signing secret
    pub secret: String,
    /// Subscribed event types
    pub events: Vec<String>,
    /// Whether the webhook is active
    pub active: bool,
    /// Creation timestamp (RFC 3339)
    pub created_at: String,
    /// Last update timestamp (RFC 3339)
    pub updated_at: String,
}

const fn default_true() -> bool {
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
    let Ok(parsed) = url::Url::parse(url) else {
        return Some("Invalid URL format");
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

    let Some(host) = parsed.host_str() else {
        return Some("URL must have a host");
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
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_ip(&ip)
    {
        return Some("URL must not target private/reserved IP ranges");
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
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
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
#[utoipa::path(
    get,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    summary = "List all webhooks",
    description = "Returns all configured webhooks. Admin only.",
    responses(
        (status = 200, description = "List of webhooks"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn list_webhooks(
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
            let responses: Vec<WebhookResponse> =
                webhooks.iter().map(WebhookResponse::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(responses)))
        }
        Err(e) => {
            tracing::error!("Failed to list webhooks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list webhooks",
                )),
            )
        }
    }
}

/// `POST /api/v1/webhooks` — create a new webhook (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/webhooks",
    tag = "Webhooks",
    summary = "Create a webhook",
    description = "Register a new webhook endpoint. URL must be HTTPS and pass SSRF validation. Admin only.",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn create_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateWebhookRequest>,
) -> (StatusCode, Json<ApiResponse<WebhookResponse>>) {
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
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create webhook",
            )),
        );
    }
    drop(state_guard);

    tracing::info!("Created webhook {} -> {}", webhook.id, webhook.url);

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(WebhookResponse::from(&webhook))),
    )
}

/// `PUT /api/v1/webhooks/{id}` — update a webhook (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    summary = "Update a webhook",
    description = "Update an existing webhook's URL, events, or active status. Admin only.",
    params(("id" = String, Path, description = "Webhook ID")),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Webhook updated"),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Webhook not found"),
    )
)]
pub async fn update_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateWebhookRequest>,
) -> (StatusCode, Json<ApiResponse<WebhookResponse>>) {
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
                    Json(ApiResponse::error("VALIDATION_ERROR", "Invalid event type")),
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
    drop(state_guard);

    tracing::info!("Updated webhook {}", webhook.id);

    (
        StatusCode::OK,
        Json(ApiResponse::success(WebhookResponse::from(&webhook))),
    )
}

/// `DELETE /api/v1/webhooks/{id}` — delete a webhook (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/webhooks/{id}",
    tag = "Webhooks",
    summary = "Delete a webhook",
    description = "Permanently remove a webhook. Admin only.",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Webhook deleted"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Webhook not found"),
    )
)]
pub async fn delete_webhook(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
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
#[utoipa::path(
    post,
    path = "/api/v1/webhooks/{id}/test",
    tag = "Webhooks",
    summary = "Send test event",
    description = "Deliver a test payload to verify webhook connectivity. Admin only.",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Test result (delivered or error details)"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Webhook not found"),
    )
)]
pub async fn test_webhook(
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
    drop(state_guard);

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
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
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

            let max_attempts = 3u32;
            let mut delay = std::time::Duration::from_secs(2);

            for attempt in 1..=max_attempts {
                match client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("X-Webhook-Signature", &signature)
                    .body(body.clone())
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        tracing::info!(
                            "Webhook {} delivered event '{}' to {} — HTTP {} (attempt {}/{})",
                            webhook.id,
                            event,
                            url,
                            resp.status(),
                            attempt,
                            max_attempts
                        );
                        return;
                    }
                    Ok(resp) => {
                        tracing::warn!(
                            "Webhook {} event '{}' to {} returned HTTP {} (attempt {}/{})",
                            webhook.id,
                            event,
                            url,
                            resp.status(),
                            attempt,
                            max_attempts
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Webhook {} failed to deliver '{}' to {}: {} (attempt {}/{})",
                            webhook.id,
                            event,
                            url,
                            e,
                            attempt,
                            max_attempts
                        );
                    }
                }

                if attempt < max_attempts {
                    tokio::time::sleep(delay).await;
                    delay *= 4; // exponential backoff: 2s, 8s
                }
            }

            tracing::error!(
                "Webhook {} delivery exhausted all {} attempts for event '{}' to {}",
                webhook.id,
                max_attempts,
                event,
                url
            );
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SSRF Validation ──────────────────────────────────────────────────

    #[test]
    fn test_ssrf_rejects_private_10_range() {
        let err = validate_webhook_url("https://10.0.0.1/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_rejects_private_172_range() {
        let err = validate_webhook_url("https://172.16.0.1/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));

        // 172.31.x.x is still in the /12 range
        let err2 = validate_webhook_url("https://172.31.255.255/hook");
        assert_eq!(err2, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_rejects_private_192_168() {
        let err = validate_webhook_url("https://192.168.1.1/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_rejects_loopback() {
        let err = validate_webhook_url("https://127.0.0.1/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));

        let err2 = validate_webhook_url("https://127.0.0.2/hook");
        assert_eq!(err2, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_rejects_link_local() {
        let err = validate_webhook_url("https://169.254.1.1/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_rejects_unspecified() {
        let err = validate_webhook_url("https://0.0.0.0/hook");
        assert_eq!(err, Some("URL must not target private/reserved IP ranges"));
    }

    #[test]
    fn test_ssrf_allows_public_ip() {
        let result = validate_webhook_url("https://93.184.216.34/hook");
        assert!(result.is_none(), "Public IP should be allowed");
    }

    #[test]
    fn test_ssrf_allows_public_domain() {
        let result = validate_webhook_url("https://hooks.example.com/webhook");
        assert!(result.is_none(), "Public domain should be allowed");
    }

    #[test]
    fn test_ssrf_rejects_invalid_url() {
        let err = validate_webhook_url("not a url");
        assert_eq!(err, Some("Invalid URL format"));
    }

    #[test]
    fn test_ssrf_rejects_ftp_scheme() {
        let err = validate_webhook_url("ftp://example.com/hook");
        assert!(err.is_some(), "Non-HTTPS/HTTP schemes should be rejected");
    }

    #[test]
    fn test_ssrf_ipv6_loopback_is_private() {
        // Verify is_private_ip correctly identifies IPv6 loopback
        use std::net::Ipv6Addr;
        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn test_ssrf_172_15_is_public() {
        // 172.15.x.x is NOT in the 172.16.0.0/12 range
        let result = validate_webhook_url("https://172.15.0.1/hook");
        assert!(result.is_none(), "172.15.x.x is a public range");
    }

    #[test]
    fn test_ssrf_172_32_is_public() {
        // 172.32.x.x is NOT in the 172.16.0.0/12 range
        let result = validate_webhook_url("https://172.32.0.1/hook");
        assert!(result.is_none(), "172.32.x.x is a public range");
    }

    // ── is_private_ip unit tests ─────────────────────────────────────────

    #[test]
    fn test_is_private_ip_covers_all_ranges() {
        use std::net::Ipv4Addr;

        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::UNSPECIFIED)));

        // Public IPs
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 15, 0, 1))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 32, 0, 1))));
    }

    #[test]
    fn test_is_private_ip_v6() {
        use std::net::Ipv6Addr;

        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
        // A public IPv6 should pass
        assert!(!is_private_ip(&IpAddr::V6(
            "2607:f8b0:4004:800::200e".parse().unwrap()
        )));
    }

    // ── HMAC Signature ───────────────────────────────────────────────────

    #[test]
    fn test_compute_signature_format() {
        let sig = compute_signature("secret", "body");
        assert!(
            sig.starts_with("sha256="),
            "Signature must start with sha256="
        );
        // sha256= prefix + 64 hex chars
        assert_eq!(sig.len(), 7 + 64);
    }

    #[test]
    fn test_compute_signature_deterministic() {
        let sig1 = compute_signature("key", "data");
        let sig2 = compute_signature("key", "data");
        assert_eq!(sig1, sig2, "Same key+body must produce same signature");
    }

    #[test]
    fn test_compute_signature_different_keys() {
        let sig1 = compute_signature("key1", "data");
        let sig2 = compute_signature("key2", "data");
        assert_ne!(
            sig1, sig2,
            "Different keys must produce different signatures"
        );
    }

    #[test]
    fn test_compute_signature_different_bodies() {
        let sig1 = compute_signature("key", "body1");
        let sig2 = compute_signature("key", "body2");
        assert_ne!(
            sig1, sig2,
            "Different bodies must produce different signatures"
        );
    }

    #[test]
    fn test_compute_signature_known_vector() {
        // Verify against a known HMAC-SHA256 test vector
        let sig = compute_signature("secret", "hello");
        // HMAC-SHA256("secret", "hello") = 88aab3ede8d3adf94d26ab90d3bafd4a2083070c3bcce9c014ee04a443847c0b
        assert_eq!(
            sig,
            "sha256=88aab3ede8d3adf94d26ab90d3bafd4a2083070c3bcce9c014ee04a443847c0b"
        );
    }

    // ── Generate Secret ──────────────────────────────────────────────────

    #[test]
    fn test_generate_secret_format() {
        let secret = generate_secret();
        assert!(
            secret.starts_with("whsec_"),
            "Secret must start with whsec_"
        );
        // whsec_ prefix + 64 hex chars
        assert_eq!(secret.len(), 6 + 64);
    }

    #[test]
    fn test_generate_secret_unique() {
        let s1 = generate_secret();
        let s2 = generate_secret();
        assert_ne!(s1, s2, "Two generated secrets must be different");
    }

    // ── Request Serde ────────────────────────────────────────────────────

    #[test]
    fn test_create_webhook_request_deserialize() {
        let json = r#"{"url": "https://example.com/hook", "events": ["booking.created"]}"#;
        let req: CreateWebhookRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.url, "https://example.com/hook");
        assert_eq!(req.events, vec!["booking.created"]);
        assert!(req.active, "active should default to true");
    }

    #[test]
    fn test_create_webhook_request_active_false() {
        let json = r#"{"url": "https://example.com/hook", "events": ["test"], "active": false}"#;
        let req: CreateWebhookRequest = serde_json::from_str(json).unwrap();
        assert!(!req.active);
    }

    #[test]
    fn test_update_webhook_request_partial() {
        let json = r#"{"active": false}"#;
        let req: UpdateWebhookRequest = serde_json::from_str(json).unwrap();
        assert!(req.url.is_none());
        assert!(req.events.is_none());
        assert_eq!(req.active, Some(false));
        assert!(!req.regenerate_secret);
    }

    #[test]
    fn test_webhook_response_from_webhook() {
        let wh = Webhook {
            id: Uuid::new_v4(),
            url: "https://test.io/hook".to_string(),
            secret: "whsec_abc".to_string(),
            events: vec!["test".to_string()],
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let resp = WebhookResponse::from(&wh);
        assert_eq!(resp.url, "https://test.io/hook");
        assert_eq!(resp.secret, "whsec_abc");
        assert!(resp.active);
        assert_eq!(resp.events, vec!["test"]);
    }

    // ── VALID_EVENTS ─────────────────────────────────────────────────────

    #[test]
    fn test_valid_events_contains_expected() {
        assert!(VALID_EVENTS.contains(&"booking.created"));
        assert!(VALID_EVENTS.contains(&"booking.cancelled"));
        assert!(VALID_EVENTS.contains(&"user.created"));
        assert!(VALID_EVENTS.contains(&"user.deleted"));
        assert!(VALID_EVENTS.contains(&"lot.created"));
        assert!(VALID_EVENTS.contains(&"test"));
        assert!(!VALID_EVENTS.contains(&"nonexistent.event"));
    }
}
