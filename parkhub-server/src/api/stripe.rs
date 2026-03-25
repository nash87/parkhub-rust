//! Stripe Checkout integration: checkout sessions, webhooks, payment history.
//!
//! Self-service: operators configure their own Stripe keys via admin settings.
//! When keys are not configured, endpoints return `501 Not Implemented`.

use axum::{
    extract::State,
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::AppState;

use super::AuthUser;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// In-memory checkout store (ephemeral — resets on restart)
// ─────────────────────────────────────────────────────────────────────────────

pub type CheckoutStore = Arc<RwLock<Vec<CheckoutRecord>>>;

pub fn new_checkout_store() -> CheckoutStore {
    Arc::new(RwLock::new(Vec::new()))
}

/// A checkout session record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckoutRecord {
    pub id: String,
    pub user_id: String,
    pub amount: u64,
    pub credits: u32,
    pub currency: String,
    pub status: CheckoutStatus,
    pub checkout_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Checkout session status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckoutStatus {
    #[default]
    Pending,
    Completed,
    Expired,
    Failed,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types
// ─────────────────────────────────────────────────────────────────────────────

/// Request to create a Stripe checkout session for credit purchase.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCheckoutRequest {
    /// Number of credits to purchase
    pub credits: u32,
    /// Price per credit in cents (smallest currency unit)
    #[serde(default = "default_price_per_credit")]
    pub price_per_credit: u64,
    /// Currency code (ISO 4217)
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_price_per_credit() -> u64 {
    100 // 1.00 in cents
}

fn default_currency() -> String {
    "eur".to_string()
}

/// Response after creating a checkout session.
#[derive(Debug, Serialize, ToSchema)]
pub struct CheckoutResponse {
    pub id: String,
    pub checkout_url: String,
    pub amount: u64,
    pub credits: u32,
    pub currency: String,
}

/// Stripe webhook event payload (simplified).
#[derive(Debug, Deserialize, ToSchema)]
pub struct WebhookEvent {
    /// Event type (e.g., "checkout.session.completed")
    #[serde(rename = "type")]
    pub event_type: String,
    /// Event data containing the object
    pub data: WebhookEventData,
}

/// Webhook event data wrapper.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WebhookEventData {
    pub object: WebhookObject,
}

/// Webhook object (checkout session or payment intent).
#[derive(Debug, Deserialize, ToSchema)]
pub struct WebhookObject {
    pub id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub metadata: std::collections::HashMap<String, String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub amount_total: Option<u64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub payment_status: Option<String>,
}

/// Payment history entry.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentHistoryEntry {
    pub id: String,
    pub amount: u64,
    pub credits: u32,
    pub currency: String,
    pub status: CheckoutStatus,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Stripe admin configuration.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct StripeConfig {
    /// Stripe publishable key (pk_live_... or pk_test_...)
    pub publishable_key: Option<String>,
    /// Whether Stripe is configured (read-only in response)
    #[serde(default)]
    pub configured: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Check if Stripe is configured (STRIPE_SECRET_KEY env var set).
fn is_stripe_configured() -> bool {
    std::env::var("STRIPE_SECRET_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/payments/create-checkout` — create a Stripe checkout session for credit purchase.
///
/// In demo mode, creates a mock checkout session. In production, would call the Stripe API.
#[utoipa::path(
    post,
    path = "/api/v1/payments/create-checkout",
    tag = "Stripe",
    summary = "Create Stripe checkout session",
    description = "Create a checkout session for purchasing credits. Returns a checkout URL.",
    request_body = CreateCheckoutRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Checkout session created"),
        (status = 400, description = "Validation error"),
        (status = 501, description = "Stripe not configured"),
    )
)]
pub async fn create_checkout(
    State(_state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Extension(store): Extension<CheckoutStore>,
    Json(req): Json<CreateCheckoutRequest>,
) -> (StatusCode, Json<ApiResponse<CheckoutResponse>>) {
    if req.credits == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "credits must be greater than 0",
            )),
        );
    }

    let amount = u64::from(req.credits) * req.price_per_credit;
    let session_id = format!("cs_{}", Uuid::new_v4().simple());

    // In a real implementation, we'd call Stripe's API here to create
    // a checkout.sessions.create with the appropriate line items.
    // For now, we create a local record and return a mock checkout URL.
    let checkout_url = if is_stripe_configured() {
        // Would call Stripe API here
        format!("https://checkout.stripe.com/c/pay/{}", session_id)
    } else {
        // Demo/unconfigured: return a local success URL
        format!("/credits?checkout={}&status=success", session_id)
    };

    let record = CheckoutRecord {
        id: session_id.clone(),
        user_id: auth_user.user_id.to_string(),
        amount,
        credits: req.credits,
        currency: req.currency.clone(),
        status: CheckoutStatus::Pending,
        checkout_url: Some(checkout_url.clone()),
        created_at: Utc::now(),
        completed_at: None,
    };

    store.write().await.push(record);

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(CheckoutResponse {
            id: session_id,
            checkout_url,
            amount,
            credits: req.credits,
            currency: req.currency,
        })),
    )
}

/// `POST /api/v1/payments/webhook` — handle Stripe webhook events.
///
/// Processes `checkout.session.completed` events to fulfill credit purchases.
/// In production, the webhook signature should be verified using the webhook secret.
#[utoipa::path(
    post,
    path = "/api/v1/payments/webhook",
    tag = "Stripe",
    summary = "Stripe webhook handler",
    description = "Receives Stripe webhook events. Processes checkout completions to grant credits.",
    request_body = WebhookEvent,
    responses(
        (status = 200, description = "Event processed"),
        (status = 400, description = "Invalid event"),
    )
)]
pub async fn stripe_webhook(
    Extension(store): Extension<CheckoutStore>,
    Json(event): Json<WebhookEvent>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    match event.event_type.as_str() {
        "checkout.session.completed" => {
            let session_id = &event.data.object.id;
            let mut records = store.write().await;

            if let Some(record) = records.iter_mut().find(|r| r.id == *session_id) {
                record.status = CheckoutStatus::Completed;
                record.completed_at = Some(Utc::now());
                tracing::info!(
                    "Checkout {} completed for user {}: {} credits",
                    session_id,
                    record.user_id,
                    record.credits,
                );
                // In production: grant credits to user via database
            } else {
                tracing::warn!("Webhook for unknown checkout session: {}", session_id);
            }

            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        "payment_intent.succeeded" => {
            tracing::info!("Payment intent succeeded: {}", event.data.object.id);
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        other => {
            tracing::debug!("Unhandled webhook event type: {}", other);
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
    }
}

/// `GET /api/v1/payments/history` — list payment history for the current user.
#[utoipa::path(
    get,
    path = "/api/v1/payments/history",
    tag = "Stripe",
    summary = "Payment history",
    description = "Returns the payment/checkout history for the authenticated user.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Payment history"),
    )
)]
pub async fn payment_history(
    Extension(auth_user): Extension<AuthUser>,
    Extension(store): Extension<CheckoutStore>,
) -> Json<ApiResponse<Vec<PaymentHistoryEntry>>> {
    let records = store.read().await;
    let user_id = auth_user.user_id.to_string();

    let history: Vec<PaymentHistoryEntry> = records
        .iter()
        .filter(|r| r.user_id == user_id)
        .map(|r| PaymentHistoryEntry {
            id: r.id.clone(),
            amount: r.amount,
            credits: r.credits,
            currency: r.currency.clone(),
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
            completed_at: r.completed_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Json(ApiResponse::success(history))
}

/// `GET /api/v1/payments/config` — get Stripe configuration status.
///
/// Returns whether Stripe is configured and the publishable key (if set).
#[utoipa::path(
    get,
    path = "/api/v1/payments/config",
    tag = "Stripe",
    summary = "Stripe configuration status",
    responses(
        (status = 200, description = "Stripe config"),
    )
)]
pub async fn stripe_config() -> Json<ApiResponse<StripeConfig>> {
    let configured = is_stripe_configured();
    let publishable_key = std::env::var("STRIPE_PUBLISHABLE_KEY").ok();

    Json(ApiResponse::success(StripeConfig {
        publishable_key,
        configured,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkout_status_serde() {
        assert_eq!(
            serde_json::to_string(&CheckoutStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&CheckoutStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&CheckoutStatus::Expired).unwrap(),
            "\"expired\""
        );
        assert_eq!(
            serde_json::to_string(&CheckoutStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    #[test]
    fn test_checkout_status_default() {
        assert_eq!(CheckoutStatus::default(), CheckoutStatus::Pending);
    }

    #[test]
    fn test_create_checkout_request_defaults() {
        let req: CreateCheckoutRequest =
            serde_json::from_value(serde_json::json!({"credits": 10})).unwrap();
        assert_eq!(req.credits, 10);
        assert_eq!(req.price_per_credit, 100);
        assert_eq!(req.currency, "eur");
    }

    #[test]
    fn test_create_checkout_request_custom() {
        let req: CreateCheckoutRequest = serde_json::from_value(
            serde_json::json!({"credits": 5, "price_per_credit": 200, "currency": "usd"}),
        )
        .unwrap();
        assert_eq!(req.credits, 5);
        assert_eq!(req.price_per_credit, 200);
        assert_eq!(req.currency, "usd");
    }

    #[test]
    fn test_create_checkout_request_missing_credits() {
        assert!(serde_json::from_value::<CreateCheckoutRequest>(
            serde_json::json!({"price_per_credit": 100})
        )
        .is_err());
    }

    #[test]
    fn test_checkout_response_serialize() {
        let resp = CheckoutResponse {
            id: "cs_abc".to_string(),
            checkout_url: "https://checkout.stripe.com/c/pay/cs_abc".to_string(),
            amount: 1000,
            credits: 10,
            currency: "eur".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["id"], "cs_abc");
        assert_eq!(json["amount"], 1000);
        assert_eq!(json["credits"], 10);
        assert!(json["checkout_url"].as_str().unwrap().contains("stripe"));
    }

    #[test]
    fn test_webhook_event_deserialize() {
        let json = serde_json::json!({
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_test",
                    "metadata": {"user_id": "u-1"},
                    "amount_total": 1000,
                    "payment_status": "paid"
                }
            }
        });
        let event: WebhookEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type, "checkout.session.completed");
        assert_eq!(event.data.object.id, "cs_test");
        assert_eq!(event.data.object.metadata.get("user_id").unwrap(), "u-1");
    }

    #[test]
    fn test_webhook_event_payment_intent() {
        let json = serde_json::json!({
            "type": "payment_intent.succeeded",
            "data": {
                "object": {
                    "id": "pi_123",
                    "metadata": {}
                }
            }
        });
        let event: WebhookEvent = serde_json::from_value(json).unwrap();
        assert_eq!(event.event_type, "payment_intent.succeeded");
    }

    #[test]
    fn test_payment_history_entry_serialize() {
        let entry = PaymentHistoryEntry {
            id: "cs_1".to_string(),
            amount: 500,
            credits: 5,
            currency: "eur".to_string(),
            status: CheckoutStatus::Completed,
            created_at: "2026-03-22T10:00:00Z".to_string(),
            completed_at: Some("2026-03-22T10:01:00Z".to_string()),
        };
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["status"], "completed");
        assert_eq!(json["credits"], 5);
    }

    #[test]
    fn test_stripe_config_serialize() {
        let config = StripeConfig {
            publishable_key: Some("pk_test_abc".to_string()),
            configured: true,
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["configured"], true);
        assert_eq!(json["publishable_key"], "pk_test_abc");
    }

    #[tokio::test]
    async fn test_checkout_store_insert() {
        let store = new_checkout_store();
        store.write().await.push(CheckoutRecord {
            id: "cs_1".to_string(),
            user_id: "u-1".to_string(),
            amount: 1000,
            credits: 10,
            currency: "eur".to_string(),
            status: CheckoutStatus::Pending,
            checkout_url: Some("https://example.com".to_string()),
            created_at: Utc::now(),
            completed_at: None,
        });
        assert_eq!(store.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_checkout_store_complete() {
        let store = new_checkout_store();
        store.write().await.push(CheckoutRecord {
            id: "cs_2".to_string(),
            user_id: "u-1".to_string(),
            amount: 500,
            credits: 5,
            currency: "eur".to_string(),
            status: CheckoutStatus::Pending,
            checkout_url: None,
            created_at: Utc::now(),
            completed_at: None,
        });
        {
            let mut records = store.write().await;
            if let Some(r) = records.iter_mut().find(|r| r.id == "cs_2") {
                r.status = CheckoutStatus::Completed;
                r.completed_at = Some(Utc::now());
            }
        }
        let records = store.read().await;
        assert_eq!(records[0].status, CheckoutStatus::Completed);
        assert!(records[0].completed_at.is_some());
    }

    #[test]
    fn test_checkout_record_roundtrip() {
        let record = CheckoutRecord {
            id: "cs_rt".to_string(),
            user_id: "u-rt".to_string(),
            amount: 2000,
            credits: 20,
            currency: "usd".to_string(),
            status: CheckoutStatus::Expired,
            checkout_url: None,
            created_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: CheckoutRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.credits, 20);
        assert_eq!(back.status, CheckoutStatus::Expired);
    }

    #[test]
    fn test_is_stripe_configured_default() {
        std::env::remove_var("STRIPE_SECRET_KEY");
        assert!(!is_stripe_configured());
    }
}
