//! Stripe payment integration stub.
//!
//! Provides mock payment intent creation, confirmation, and status endpoints.
//! Gated behind the `PAYMENT_ENABLED` environment variable (disabled by default).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Feature flag
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `true` when the `PAYMENT_ENABLED` env var is set to `"true"` or `"1"`.
fn payments_enabled() -> bool {
    std::env::var("PAYMENT_ENABLED")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Payment status for Stripe-style intents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StripePaymentStatus {
    Pending,
    Succeeded,
    Failed,
    Refunded,
}

impl Default for StripePaymentStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// A mock Stripe PaymentIntent (internal model, not yet persisted).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PaymentIntent {
    pub id: String,
    pub booking_id: Uuid,
    pub amount_cents: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── Request / Response DTOs ────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentIntentRequest {
    pub booking_id: Uuid,
    pub amount_cents: u64,
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
    pub booking_id: Uuid,
    pub amount_cents: u64,
    pub currency: String,
    pub status: StripePaymentStatus,
    pub client_secret: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentStatusResponse {
    pub booking_id: Uuid,
    pub status: StripePaymentStatus,
    pub payment_intent_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/payments/create-intent` — create a mock payment intent.
#[utoipa::path(
    post,
    path = "/api/v1/payments/create-intent",
    tag = "Payments",
    request_body = CreatePaymentIntentRequest,
    responses(
        (status = 201, description = "Payment intent created", body = PaymentIntentResponse),
        (status = 503, description = "Payments disabled"),
    ),
)]
pub async fn create_payment_intent(
    State(_state): State<SharedState>,
    Json(req): Json<CreatePaymentIntentRequest>,
) -> impl IntoResponse {
    if !payments_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "payments_disabled" })),
        )
            .into_response();
    }

    let intent_id = format!("pi_{}", Uuid::new_v4().simple());
    let client_secret = format!("pi_secret_{}", Uuid::new_v4().simple());

    let resp = PaymentIntentResponse {
        id: intent_id,
        booking_id: req.booking_id,
        amount_cents: req.amount_cents,
        currency: req.currency,
        status: StripePaymentStatus::Pending,
        client_secret,
    };

    (StatusCode::CREATED, Json(serde_json::json!(resp))).into_response()
}

/// `POST /api/v1/payments/confirm` — confirm (mock) a payment intent.
#[utoipa::path(
    post,
    path = "/api/v1/payments/confirm",
    tag = "Payments",
    request_body = ConfirmPaymentRequest,
    responses(
        (status = 200, description = "Payment confirmed", body = PaymentIntentResponse),
        (status = 503, description = "Payments disabled"),
    ),
)]
pub async fn confirm_payment(
    State(_state): State<SharedState>,
    Json(req): Json<ConfirmPaymentRequest>,
) -> impl IntoResponse {
    if !payments_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "payments_disabled" })),
        )
            .into_response();
    }

    let resp = PaymentIntentResponse {
        id: req.payment_intent_id,
        booking_id: Uuid::nil(),
        amount_cents: 0,
        currency: "eur".to_string(),
        status: StripePaymentStatus::Succeeded,
        client_secret: String::new(),
    };

    (StatusCode::OK, Json(serde_json::json!(resp))).into_response()
}

/// `GET /api/v1/payments/{booking_id}/status` — payment status for a booking.
#[utoipa::path(
    get,
    path = "/api/v1/payments/{booking_id}/status",
    tag = "Payments",
    params(("booking_id" = Uuid, Path, description = "Booking UUID")),
    responses(
        (status = 200, description = "Payment status", body = PaymentStatusResponse),
        (status = 503, description = "Payments disabled"),
    ),
)]
pub async fn payment_status(
    State(_state): State<SharedState>,
    Path(booking_id): Path<Uuid>,
) -> impl IntoResponse {
    if !payments_enabled() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "payments_disabled" })),
        )
            .into_response();
    }

    let resp = PaymentStatusResponse {
        booking_id,
        status: StripePaymentStatus::Pending,
        payment_intent_id: None,
    };

    (StatusCode::OK, Json(serde_json::json!(resp))).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_status_serde_roundtrip() {
        let cases = [
            (StripePaymentStatus::Pending, "\"pending\""),
            (StripePaymentStatus::Succeeded, "\"succeeded\""),
            (StripePaymentStatus::Failed, "\"failed\""),
            (StripePaymentStatus::Refunded, "\"refunded\""),
        ];
        for (variant, expected) in &cases {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(&json, expected);
            let back: StripePaymentStatus = serde_json::from_str(expected).unwrap();
            assert_eq!(&back, variant);
        }
    }

    #[test]
    fn test_payment_status_default_is_pending() {
        assert_eq!(StripePaymentStatus::default(), StripePaymentStatus::Pending);
    }

    #[test]
    fn test_create_intent_request_deserializes() {
        let json = serde_json::json!({
            "booking_id": "550e8400-e29b-41d4-a716-446655440000",
            "amount_cents": 550
        });
        let req: CreatePaymentIntentRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.amount_cents, 550);
        assert_eq!(req.currency, "eur");
    }

    #[test]
    fn test_create_intent_request_custom_currency() {
        let json = serde_json::json!({
            "booking_id": "550e8400-e29b-41d4-a716-446655440000",
            "amount_cents": 1200,
            "currency": "usd"
        });
        let req: CreatePaymentIntentRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.currency, "usd");
    }

    #[test]
    fn test_confirm_request_deserializes() {
        let json = serde_json::json!({ "payment_intent_id": "pi_abc123" });
        let req: ConfirmPaymentRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.payment_intent_id, "pi_abc123");
    }

    #[test]
    fn test_payment_intent_response_serializes() {
        let resp = PaymentIntentResponse {
            id: "pi_test".to_string(),
            booking_id: Uuid::nil(),
            amount_cents: 999,
            currency: "eur".to_string(),
            status: StripePaymentStatus::Succeeded,
            client_secret: "pi_secret_test".to_string(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "succeeded");
        assert_eq!(json["amount_cents"], 999);
        assert_eq!(json["client_secret"], "pi_secret_test");
    }

    #[test]
    fn test_payment_status_response_serializes() {
        let booking_id = Uuid::new_v4();
        let resp = PaymentStatusResponse {
            booking_id,
            status: StripePaymentStatus::Pending,
            payment_intent_id: None,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "pending");
        assert!(json["payment_intent_id"].is_null());
    }

    #[test]
    fn test_payment_status_response_with_intent_id() {
        let resp = PaymentStatusResponse {
            booking_id: Uuid::nil(),
            status: StripePaymentStatus::Failed,
            payment_intent_id: Some("pi_xyz".to_string()),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "failed");
        assert_eq!(json["payment_intent_id"], "pi_xyz");
    }

    #[test]
    fn test_payments_disabled_by_default() {
        std::env::remove_var("PAYMENT_ENABLED");
        assert!(!payments_enabled());
    }

    #[test]
    fn test_payments_enabled_with_true() {
        std::env::set_var("PAYMENT_ENABLED", "true");
        assert!(payments_enabled());
        std::env::remove_var("PAYMENT_ENABLED");
    }

    #[test]
    fn test_payments_enabled_with_one() {
        std::env::set_var("PAYMENT_ENABLED", "1");
        assert!(payments_enabled());
        std::env::remove_var("PAYMENT_ENABLED");
    }

    #[test]
    fn test_payments_disabled_with_false() {
        std::env::set_var("PAYMENT_ENABLED", "false");
        assert!(!payments_enabled());
        std::env::remove_var("PAYMENT_ENABLED");
    }
}
