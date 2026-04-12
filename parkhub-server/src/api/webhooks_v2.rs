//! Webhooks v2 — Outgoing Event Subscriptions with delivery tracking.
//!
//! Enhanced webhook system with delivery logs, retry logic, and HMAC-SHA256 signing.
//!
//! Endpoints:
//! - `GET    /api/v1/admin/webhooks`              — list subscriptions
//! - `POST   /api/v1/admin/webhooks`              — create subscription
//! - `PUT    /api/v1/admin/webhooks/{id}`          — update subscription
//! - `DELETE /api/v1/admin/webhooks/{id}`          — delete subscription
//! - `POST   /api/v1/admin/webhooks/{id}/test`     — send test event
//! - `GET    /api/v1/admin/webhooks/{id}/deliveries` — delivery log

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported webhook event types.
pub const WEBHOOK_V2_EVENTS: &[&str] = &[
    "booking.created",
    "booking.cancelled",
    "user.registered",
    "lot.full",
    "payment.completed",
];

/// Max retry attempts for failed deliveries.
const MAX_RETRIES: u8 = 3;

/// Webhook v2 subscription stored in settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookV2 {
    pub id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Delivery log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryEntry {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: String,
    pub status_code: Option<u16>,
    pub success: bool,
    pub attempt: u8,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub delivered_at: DateTime<Utc>,
}

/// Response type for webhook list.
#[derive(Debug, Serialize)]
pub struct WebhookV2Response {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub active: bool,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&WebhookV2> for WebhookV2Response {
    fn from(w: &WebhookV2) -> Self {
        Self {
            id: w.id.to_string(),
            url: w.url.clone(),
            secret: w.secret.clone(),
            events: w.events.clone(),
            active: w.active,
            description: w.description.clone(),
            created_at: w.created_at.to_rfc3339(),
            updated_at: w.updated_at.to_rfc3339(),
        }
    }
}

/// Delivery log response.
#[derive(Debug, Serialize)]
pub struct DeliveryResponse {
    pub id: String,
    pub event_type: String,
    pub status_code: Option<u16>,
    pub success: bool,
    pub attempt: u8,
    pub error: Option<String>,
    pub delivered_at: String,
}

impl From<&DeliveryEntry> for DeliveryResponse {
    fn from(d: &DeliveryEntry) -> Self {
        Self {
            id: d.id.to_string(),
            event_type: d.event_type.clone(),
            status_code: d.status_code,
            success: d.success,
            attempt: d.attempt,
            error: d.error.clone(),
            delivered_at: d.delivered_at.to_rfc3339(),
        }
    }
}

/// Request to create a webhook subscription.
#[derive(Debug, Deserialize)]
pub struct CreateWebhookV2Request {
    pub url: String,
    pub events: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub active: bool,
}

/// Request to update a webhook subscription.
#[derive(Debug, Deserialize)]
pub struct UpdateWebhookV2Request {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub active: Option<bool>,
    pub description: Option<String>,
    #[serde(default)]
    pub regenerate_secret: bool,
}

const fn default_true() -> bool {
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// HMAC signing
// ─────────────────────────────────────────────────────────────────────────────

/// Generate a cryptographically random secret for HMAC signing.
fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    format!("whsec_{}", hex::encode(bytes))
}

/// Compute HMAC-SHA256 signature for a payload.
pub fn compute_signature(secret: &str, payload: &[u8]) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

/// Validate a webhook URL.
fn validate_url(url: &str) -> Option<&'static str> {
    let Ok(parsed) = url::Url::parse(url) else {
        return Some("Invalid URL format");
    };
    if parsed.scheme() != "https" {
        #[cfg(debug_assertions)]
        {
            let host = parsed.host_str().unwrap_or("");
            if parsed.scheme() != "http" || (host != "localhost" && host != "127.0.0.1") {
                return Some("URL must use HTTPS");
            }
        }
        #[cfg(not(debug_assertions))]
        return Some("URL must use HTTPS");
    }
    if parsed.host_str().is_none() {
        return Some("URL must have a host");
    }
    None
}

/// Validate event types.
fn validate_events(events: &[String]) -> Option<String> {
    for ev in events {
        if !WEBHOOK_V2_EVENTS.contains(&ev.as_str()) {
            return Some(format!("Unknown event type: {ev}"));
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence helpers (stored in settings as JSON)
// ─────────────────────────────────────────────────────────────────────────────

const WEBHOOKS_KEY: &str = "webhooks_v2";
const DELIVERIES_KEY: &str = "webhooks_v2_deliveries";

async fn load_webhooks(state: &crate::AppState) -> Vec<WebhookV2> {
    match state.db.get_setting(WEBHOOKS_KEY).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => Vec::new(),
    }
}

async fn save_webhooks(state: &crate::AppState, webhooks: &[WebhookV2]) {
    let json = serde_json::to_string(webhooks).unwrap_or_default();
    let _ = state.db.set_setting(WEBHOOKS_KEY, &json).await;
}

async fn load_deliveries(state: &crate::AppState, webhook_id: &Uuid) -> Vec<DeliveryEntry> {
    let key = format!("{DELIVERIES_KEY}:{webhook_id}");
    match state.db.get_setting(&key).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => Vec::new(),
    }
}

async fn save_delivery(state: &crate::AppState, delivery: &DeliveryEntry) {
    let key = format!("{DELIVERIES_KEY}:{}", delivery.webhook_id);
    let mut deliveries = match state.db.get_setting(&key).await {
        Ok(Some(json)) => serde_json::from_str::<Vec<DeliveryEntry>>(&json).unwrap_or_default(),
        _ => Vec::new(),
    };
    // Keep last 100 deliveries
    deliveries.push(delivery.clone());
    if deliveries.len() > 100 {
        deliveries.drain(0..deliveries.len() - 100);
    }
    let json = serde_json::to_string(&deliveries).unwrap_or_default();
    let _ = state.db.set_setting(&key, &json).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Delivery engine
// ─────────────────────────────────────────────────────────────────────────────

/// Deliver an event payload to a webhook URL with retries.
async fn deliver_event(
    state: &crate::AppState,
    webhook: &WebhookV2,
    event_type: &str,
    payload: &serde_json::Value,
) -> DeliveryEntry {
    let payload_bytes = serde_json::to_vec(payload).unwrap_or_default();
    let signature = compute_signature(&webhook.secret, &payload_bytes);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut last_entry = DeliveryEntry {
        id: Uuid::new_v4(),
        webhook_id: webhook.id,
        event_type: event_type.to_string(),
        status_code: None,
        success: false,
        attempt: 0,
        response_body: None,
        error: None,
        delivered_at: Utc::now(),
    };

    for attempt in 1..=MAX_RETRIES {
        last_entry.attempt = attempt;

        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-ParkHub-Signature", &signature)
            .header("X-ParkHub-Event", event_type)
            .header("X-ParkHub-Delivery", last_entry.id.to_string())
            .body(payload_bytes.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status().as_u16();
                last_entry.status_code = Some(status);
                last_entry.delivered_at = Utc::now();

                if resp.status().is_success() {
                    last_entry.success = true;
                    last_entry.error = None;
                    break;
                }
                let body = resp.text().await.unwrap_or_default();
                last_entry.response_body = Some(body.chars().take(500).collect());
                last_entry.error = Some(format!("HTTP {status}"));
            }
            Err(e) => {
                last_entry.error = Some(format!("Request failed: {e}"));
                last_entry.delivered_at = Utc::now();
            }
        }

        // Exponential backoff: 1s, 4s, 16s
        if attempt < MAX_RETRIES {
            let delay = std::time::Duration::from_secs(u64::from(attempt).pow(2));
            tokio::time::sleep(delay).await;
        }
    }

    save_delivery(state, &last_entry).await;
    last_entry
}

// ─────────────────────────────────────────────────────────────────────────────
// Public dispatch function (called from other modules)
// ─────────────────────────────────────────────────────────────────────────────

/// Dispatch an event to all matching webhook subscriptions.
/// Runs asynchronously — does not block the caller.
#[allow(dead_code)]
pub fn dispatch_event(state: super::SharedState, event_type: String, payload: serde_json::Value) {
    tokio::spawn(async move {
        let state_guard = state.read().await;
        let webhooks = load_webhooks(&state_guard).await;

        for wh in &webhooks {
            if wh.active && wh.events.iter().any(|e| e == &event_type) {
                deliver_event(&state_guard, wh, &event_type, &payload).await;
            }
        }
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/webhooks` — list all webhook subscriptions.
pub async fn list_webhooks_v2(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<WebhookV2Response>>> {
    let state_guard = state.read().await;
    let webhooks = load_webhooks(&state_guard).await;
    let responses: Vec<WebhookV2Response> = webhooks.iter().map(WebhookV2Response::from).collect();
    Json(ApiResponse::success(responses))
}

/// `POST /api/v1/admin/webhooks` — create a webhook subscription.
pub async fn create_webhook_v2(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateWebhookV2Request>,
) -> Result<(StatusCode, Json<ApiResponse<WebhookV2Response>>), (StatusCode, Json<ApiResponse<()>>)>
{
    // Validate URL
    if let Some(err) = validate_url(&req.url) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("VALIDATION_ERROR", err)),
        ));
    }

    // Validate events
    if req.events.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "At least one event type is required",
            )),
        ));
    }
    if let Some(err) = validate_events(&req.events) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("VALIDATION_ERROR", &err)),
        ));
    }

    let now = Utc::now();
    let webhook = WebhookV2 {
        id: Uuid::new_v4(),
        url: req.url,
        secret: generate_secret(),
        events: req.events,
        active: req.active,
        description: req.description,
        created_at: now,
        updated_at: now,
    };

    let state_guard = state.read().await;
    let mut webhooks = load_webhooks(&state_guard).await;
    webhooks.push(webhook.clone());
    save_webhooks(&state_guard, &webhooks).await;

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("webhook_v2_created:{}", webhook.id))
        .log()
        .persist(&state_guard.db)
        .await;
    drop(state_guard);

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(WebhookV2Response::from(&webhook))),
    ))
}

/// `PUT /api/v1/admin/webhooks/{id}` — update a webhook subscription.
pub async fn update_webhook_v2(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(webhook_id): Path<Uuid>,
    Json(req): Json<UpdateWebhookV2Request>,
) -> Result<Json<ApiResponse<WebhookV2Response>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;
    let mut webhooks = load_webhooks(&state_guard).await;

    let wh = webhooks
        .iter_mut()
        .find(|w| w.id == webhook_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
            )
        })?;

    if let Some(url) = &req.url {
        if let Some(err) = validate_url(url) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", err)),
            ));
        }
        wh.url = url.clone();
    }
    if let Some(events) = &req.events {
        if let Some(err) = validate_events(events) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", &err)),
            ));
        }
        wh.events = events.clone();
    }
    if let Some(active) = req.active {
        wh.active = active;
    }
    if let Some(desc) = &req.description {
        wh.description = Some(desc.clone());
    }
    if req.regenerate_secret {
        wh.secret = generate_secret();
    }
    wh.updated_at = Utc::now();

    let response = WebhookV2Response::from(&*wh);
    save_webhooks(&state_guard, &webhooks).await;

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("webhook_v2_updated:{webhook_id}"))
        .log()
        .persist(&state_guard.db)
        .await;
    drop(state_guard);

    Ok(Json(ApiResponse::success(response)))
}

/// `DELETE /api/v1/admin/webhooks/{id}` — delete a webhook subscription.
pub async fn delete_webhook_v2(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(webhook_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;
    let mut webhooks = load_webhooks(&state_guard).await;
    let initial_len = webhooks.len();
    webhooks.retain(|w| w.id != webhook_id);

    if webhooks.len() == initial_len {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
        ));
    }

    save_webhooks(&state_guard, &webhooks).await;

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("webhook_v2_deleted:{webhook_id}"))
        .log()
        .persist(&state_guard.db)
        .await;
    drop(state_guard);

    Ok(Json(ApiResponse::success(())))
}

/// `POST /api/v1/admin/webhooks/{id}/test` — send a test event.
pub async fn test_webhook_v2(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(webhook_id): Path<Uuid>,
) -> Result<Json<ApiResponse<DeliveryResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;
    let webhooks = load_webhooks(&state_guard).await;

    let wh = webhooks
        .iter()
        .find(|w| w.id == webhook_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
            )
        })?;

    let test_payload = serde_json::json!({
        "event": "test",
        "webhook_id": webhook_id.to_string(),
        "timestamp": Utc::now().to_rfc3339(),
        "data": { "message": "This is a test event from ParkHub" }
    });

    let delivery = deliver_event(&state_guard, wh, "test", &test_payload).await;
    drop(state_guard);

    Ok(Json(ApiResponse::success(DeliveryResponse::from(
        &delivery,
    ))))
}

/// `GET /api/v1/admin/webhooks/{id}/deliveries` — delivery log.
pub async fn list_deliveries_v2(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(webhook_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<DeliveryResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;

    // Verify webhook exists
    let webhooks = load_webhooks(&state_guard).await;
    if !webhooks.iter().any(|w| w.id == webhook_id) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Webhook not found")),
        ));
    }

    let deliveries = load_deliveries(&state_guard, &webhook_id).await;
    let responses: Vec<DeliveryResponse> = deliveries.iter().map(DeliveryResponse::from).collect();
    drop(state_guard);

    Ok(Json(ApiResponse::success(responses)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_signature() {
        let sig = compute_signature("whsec_abc123", b"hello world");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64); // SHA256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn test_compute_signature_deterministic() {
        let sig1 = compute_signature("secret", b"payload");
        let sig2 = compute_signature("secret", b"payload");
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_compute_signature_different_secrets() {
        let sig1 = compute_signature("secret1", b"payload");
        let sig2 = compute_signature("secret2", b"payload");
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_generate_secret_format() {
        let secret = generate_secret();
        assert!(secret.starts_with("whsec_"));
        assert_eq!(secret.len(), 6 + 64); // "whsec_" + 32 bytes hex
    }

    #[test]
    fn test_generate_secret_unique() {
        let s1 = generate_secret();
        let s2 = generate_secret();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_validate_url_valid_https() {
        assert!(validate_url("https://example.com/hooks").is_none());
    }

    #[test]
    fn test_validate_url_invalid_format() {
        let result = validate_url("not a url");
        assert!(result.is_some());
    }

    #[test]
    fn test_validate_events_valid() {
        let events = vec!["booking.created".to_string(), "lot.full".to_string()];
        assert!(validate_events(&events).is_none());
    }

    #[test]
    fn test_validate_events_unknown() {
        let events = vec!["unknown.event".to_string()];
        let err = validate_events(&events);
        assert!(err.is_some());
        assert!(err.unwrap().contains("unknown.event"));
    }

    #[test]
    fn test_webhook_v2_serialization() {
        let wh = WebhookV2 {
            id: Uuid::new_v4(),
            url: "https://example.com/hook".to_string(),
            secret: "whsec_test".to_string(),
            events: vec!["booking.created".to_string()],
            active: true,
            description: Some("Test webhook".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&wh).unwrap();
        let deserialized: WebhookV2 = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.url, "https://example.com/hook");
        assert!(deserialized.active);
    }

    #[test]
    fn test_delivery_entry_serialization() {
        let entry = DeliveryEntry {
            id: Uuid::new_v4(),
            webhook_id: Uuid::new_v4(),
            event_type: "booking.created".to_string(),
            status_code: Some(200),
            success: true,
            attempt: 1,
            response_body: None,
            error: None,
            delivered_at: Utc::now(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let de: DeliveryEntry = serde_json::from_str(&json).unwrap();
        assert!(de.success);
        assert_eq!(de.status_code, Some(200));
    }

    #[test]
    fn test_create_request_defaults() {
        let json = r#"{"url":"https://example.com","events":["booking.created"]}"#;
        let req: CreateWebhookV2Request = serde_json::from_str(json).unwrap();
        assert!(req.active); // default_true
        assert!(req.description.is_none());
    }

    #[test]
    fn test_webhook_v2_response_from() {
        let wh = WebhookV2 {
            id: Uuid::new_v4(),
            url: "https://test.com".to_string(),
            secret: "whsec_abc".to_string(),
            events: vec!["lot.full".to_string()],
            active: false,
            description: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let resp = WebhookV2Response::from(&wh);
        assert_eq!(resp.url, "https://test.com");
        assert!(!resp.active);
    }
}
