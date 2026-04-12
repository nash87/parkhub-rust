//! Stripe payment integration stub for demo/showcase.
//!
//! Provides mock payment intent creation, confirmation, and status endpoints
//! with realistic Stripe-like response shapes.
//!
//! - **Demo mode** (`DEMO_MODE=true`): always succeeds with mock data.
//! - **Production**: returns `501 Not Implemented`.
//!
//! Payments are tracked in an in-memory `HashMap` (no persistence).

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// In-memory payment store
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe in-memory payment store (ephemeral — resets on restart).
pub type PaymentStore = Arc<RwLock<HashMap<String, StoredPayment>>>;

/// Create a new empty payment store.
pub fn new_payment_store() -> PaymentStore {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Internal payment record stored in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPayment {
    pub id: String,
    pub amount: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Feature flag
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true when the server is running in demo mode (`DEMO_MODE=true`).
fn is_demo_mode() -> bool {
    std::env::var("DEMO_MODE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Payment status for Stripe-style intents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    #[default]
    Pending,
    Succeeded,
    Failed,
    Refunded,
}

// ── Request / Response DTOs ────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentIntentRequest {
    pub amount: u64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "eur".to_string()
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfirmPaymentRequest {
    pub payment_intent_id: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentIntentResponse {
    pub id: String,
    pub amount: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentStatusResponse {
    pub id: String,
    pub amount: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/payments/create-intent",
    tag = "Payments",
    request_body = CreatePaymentIntentRequest,
    responses(
        (status = 201, description = "Payment intent created", body = PaymentIntentResponse),
        (status = 501, description = "Payments not implemented (production mode)"),
    ),
)]
pub async fn create_payment_intent(
    State(_state): State<SharedState>,
    Extension(store): Extension<PaymentStore>,
    Json(req): Json<CreatePaymentIntentRequest>,
) -> impl IntoResponse {
    if !is_demo_mode() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!(ApiResponse::<()>::error(
                "NOT_IMPLEMENTED",
                "Payment processing is not available in production mode"
            ))),
        )
            .into_response();
    }
    let intent_id = format!("pi_{}", Uuid::new_v4().simple());
    let client_secret = format!("{}_secret_{}", &intent_id, Uuid::new_v4().simple());
    let now = Utc::now();
    let payment = StoredPayment {
        id: intent_id.clone(),
        amount: req.amount,
        currency: req.currency.clone(),
        status: StripePaymentStatus::Pending,
        client_secret: client_secret.clone(),
        created_at: now,
        updated_at: now,
    };
    store.write().await.insert(intent_id.clone(), payment);
    let resp = PaymentIntentResponse {
        id: intent_id,
        amount: req.amount,
        currency: req.currency,
        status: StripePaymentStatus::Pending,
        client_secret,
    };
    (StatusCode::CREATED, Json(serde_json::json!(resp))).into_response()
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/confirm",
    tag = "Payments",
    request_body = ConfirmPaymentRequest,
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentIntentResponse),
        (status = 404, description = "Payment intent not found"),
        (status = 501, description = "Payments not implemented (production mode)"),
    ),
)]
pub async fn confirm_payment(
    State(_state): State<SharedState>,
    Extension(store): Extension<PaymentStore>,
    Json(req): Json<ConfirmPaymentRequest>,
) -> impl IntoResponse {
    if !is_demo_mode() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!(ApiResponse::<()>::error(
                "NOT_IMPLEMENTED",
                "Payment processing is not available in production mode"
            ))),
        )
            .into_response();
    }
    let mut payments = store.write().await;
    let Some(payment) = payments.get_mut(&req.payment_intent_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ApiResponse::<()>::error(
                "NOT_FOUND",
                "Payment intent not found"
            ))),
        )
            .into_response();
    };
    payment.status = StripePaymentStatus::Succeeded;
    payment.updated_at = Utc::now();
    let resp = PaymentIntentResponse {
        id: payment.id.clone(),
        amount: payment.amount,
        currency: payment.currency.clone(),
        status: payment.status,
        client_secret: payment.client_secret.clone(),
    };
    drop(payments);
    (StatusCode::OK, Json(serde_json::json!(resp))).into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/{id}/status",
    tag = "Payments",
    params(("id" = String, Path, description = "Payment intent ID")),
    responses(
        (status = 200, description = "Payment status", body = PaymentStatusResponse),
        (status = 404, description = "Payment intent not found"),
        (status = 501, description = "Payments not implemented (production mode)"),
    ),
)]
pub async fn payment_status(
    State(_state): State<SharedState>,
    Extension(store): Extension<PaymentStore>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_demo_mode() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(serde_json::json!(ApiResponse::<()>::error(
                "NOT_IMPLEMENTED",
                "Payment processing is not available in production mode"
            ))),
        )
            .into_response();
    }
    let payments = store.read().await;
    let Some(payment) = payments.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!(ApiResponse::<()>::error(
                "NOT_FOUND",
                "Payment intent not found"
            ))),
        )
            .into_response();
    };
    let resp = PaymentStatusResponse {
        id: payment.id.clone(),
        amount: payment.amount,
        currency: payment.currency.clone(),
        status: payment.status,
    };
    drop(payments);
    (StatusCode::OK, Json(serde_json::json!(resp))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_payment_status_serde_roundtrip() {
        for (variant, expected) in &[
            (StripePaymentStatus::Pending, "\"pending\""),
            (StripePaymentStatus::Succeeded, "\"succeeded\""),
            (StripePaymentStatus::Failed, "\"failed\""),
            (StripePaymentStatus::Refunded, "\"refunded\""),
        ] {
            assert_eq!(&serde_json::to_string(variant).unwrap(), expected);
            assert_eq!(
                &serde_json::from_str::<StripePaymentStatus>(expected).unwrap(),
                variant
            );
        }
    }

    #[test]
    fn test_stripe_payment_status_default() {
        assert_eq!(StripePaymentStatus::default(), StripePaymentStatus::Pending);
    }

    #[test]
    fn test_stripe_payment_status_unknown_fails() {
        assert!(serde_json::from_str::<StripePaymentStatus>("\"cancelled\"").is_err());
    }

    #[test]
    fn test_create_intent_request_defaults() {
        let req: CreatePaymentIntentRequest =
            serde_json::from_value(serde_json::json!({"amount": 550})).unwrap();
        assert_eq!(req.amount, 550);
        assert_eq!(req.currency, "eur");
    }

    #[test]
    fn test_create_intent_request_custom_currency() {
        let req: CreatePaymentIntentRequest =
            serde_json::from_value(serde_json::json!({"amount": 1200, "currency": "usd"})).unwrap();
        assert_eq!(req.amount, 1200);
        assert_eq!(req.currency, "usd");
    }

    #[test]
    fn test_create_intent_request_missing_amount() {
        assert!(
            serde_json::from_value::<CreatePaymentIntentRequest>(
                serde_json::json!({"currency": "eur"})
            )
            .is_err()
        );
    }

    #[test]
    fn test_confirm_request() {
        let req: ConfirmPaymentRequest =
            serde_json::from_value(serde_json::json!({"payment_intent_id": "pi_abc"})).unwrap();
        assert_eq!(req.payment_intent_id, "pi_abc");
    }

    #[test]
    fn test_confirm_request_missing_id() {
        assert!(serde_json::from_value::<ConfirmPaymentRequest>(serde_json::json!({})).is_err());
    }

    #[test]
    fn test_payment_intent_response_serializes() {
        let resp = PaymentIntentResponse {
            id: "pi_t".into(),
            amount: 999,
            currency: "eur".into(),
            status: StripePaymentStatus::Succeeded,
            client_secret: "s".into(),
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["id"], "pi_t");
        assert_eq!(j["amount"], 999);
        assert_eq!(j["status"], "succeeded");
    }

    #[test]
    fn test_payment_status_response_serializes() {
        let resp = PaymentStatusResponse {
            id: "pi_x".into(),
            amount: 500,
            currency: "eur".into(),
            status: StripePaymentStatus::Pending,
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["id"], "pi_x");
        assert_eq!(j["status"], "pending");
    }

    // Env var tests consolidated into one to avoid parallel race conditions
    #[test]
    #[allow(unsafe_code)]
    fn test_demo_mode_env_var() {
        // Must run sequentially — env vars are process-global
        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("DEMO_MODE") };
        assert!(!is_demo_mode(), "should be disabled by default");

        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("DEMO_MODE", "true") };
        assert!(is_demo_mode(), "should be enabled with 'true'");

        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("DEMO_MODE", "1") };
        assert!(is_demo_mode(), "should be enabled with '1'");

        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::set_var("DEMO_MODE", "false") };
        assert!(!is_demo_mode(), "should be disabled with 'false'");

        // SAFETY: single-threaded test or pre-spawn context
        unsafe { std::env::remove_var("DEMO_MODE") };
    }

    #[tokio::test]
    async fn test_store_insert_retrieve() {
        let store = new_payment_store();
        let now = Utc::now();
        store.write().await.insert(
            "pi_1".into(),
            StoredPayment {
                id: "pi_1".into(),
                amount: 1000,
                currency: "eur".into(),
                status: StripePaymentStatus::Pending,
                client_secret: "s".into(),
                created_at: now,
                updated_at: now,
            },
        );
        assert_eq!(store.read().await.get("pi_1").unwrap().amount, 1000);
    }

    #[tokio::test]
    async fn test_store_confirm() {
        let store = new_payment_store();
        let now = Utc::now();
        store.write().await.insert(
            "pi_c".into(),
            StoredPayment {
                id: "pi_c".into(),
                amount: 500,
                currency: "usd".into(),
                status: StripePaymentStatus::Pending,
                client_secret: "s".into(),
                created_at: now,
                updated_at: now,
            },
        );
        {
            let mut m = store.write().await;
            let p = m.get_mut("pi_c").unwrap();
            p.status = StripePaymentStatus::Succeeded;
        }
        assert_eq!(
            store.read().await.get("pi_c").unwrap().status,
            StripePaymentStatus::Succeeded
        );
    }

    #[tokio::test]
    async fn test_store_missing() {
        assert!(new_payment_store().read().await.get("nope").is_none());
    }

    #[test]
    fn test_stored_payment_roundtrip() {
        let now = Utc::now();
        let p = StoredPayment {
            id: "pi_r".into(),
            amount: 2500,
            currency: "gbp".into(),
            status: StripePaymentStatus::Refunded,
            client_secret: "s".into(),
            created_at: now,
            updated_at: now,
        };
        let back: StoredPayment =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(back.amount, 2500);
        assert_eq!(back.status, StripePaymentStatus::Refunded);
    }
}
