//! Integration tests for booking workflow, admin endpoints, and rate limiting.
//!
//! Closes #63 (booking workflow) and #62 (admin + rate-limit tests).

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use chrono::{TimeDelta, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use crate::api::create_router;
use crate::config::ServerConfig;
use crate::db::{Database, DatabaseConfig};
use crate::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers (mirrors integration_tests.rs)
// ─────────────────────────────────────────────────────────────────────────────

async fn test_state() -> Arc<RwLock<AppState>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_config = DatabaseConfig {
        path: dir.path().to_path_buf(),
        encryption_enabled: false,
        passphrase: None,
        create_if_missing: true,
    };
    let db = Database::open(&db_config).expect("open test db");

    let config = ServerConfig {
        admin_password_hash: hash_password_for_test("admin123"),
        allow_self_registration: true,
        ..ServerConfig::default()
    };

    let state = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        db,
        mdns: None,
        scheduler: None,
        ws_events: crate::api::ws::EventBroadcaster::new(),
    }));

    {
        let guard = state.read().await;
        crate::create_admin_user(&guard.db, &guard.config)
            .await
            .expect("seed admin");
    }

    std::mem::forget(dir);
    state
}

fn router(state: Arc<RwLock<AppState>>) -> axum::Router {
    let (router, _demo) = create_router(state);
    router
}

fn hash_password_for_test(password: &str) -> String {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

async fn body_bytes(response: http::Response<Body>) -> Vec<u8> {
    use http_body_util::BodyExt;
    response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes()
        .to_vec()
}

async fn body_json(response: http::Response<Body>) -> serde_json::Value {
    let bytes = body_bytes(response).await;
    serde_json::from_slice(&bytes).expect("parse JSON")
}

/// Login as admin and return the access token.
async fn admin_token(state: Arc<RwLock<AppState>>) -> String {
    let app = router(state);
    let body = serde_json::json!({"username": "admin", "password": "admin123"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    json["data"]["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Register a new user and return their access token.
async fn register_user_token(
    state: Arc<RwLock<AppState>>,
    email: &str,
    password: &str,
) -> (String, String) {
    let app = router(state);
    let body = serde_json::json!({
        "email": email,
        "password": password,
        "password_confirmation": password,
        "name": "Test User",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    let token = json["data"]["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    let user_id = json["data"]["user"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    (token, user_id)
}

/// Create a parking lot via admin API and return (lot_id, first_slot_id).
async fn create_lot_and_get_slot(
    state: Arc<RwLock<AppState>>,
    admin_tok: &str,
) -> (String, String) {
    // Create lot
    let lot_body = serde_json::json!({
        "name": "Test Lot",
        "total_slots": 5,
        "currency": "EUR",
    });
    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::post("/api/v1/lots")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&lot_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "create lot failed");
    let json = body_json(resp).await;
    let lot_id = json["data"]["id"].as_str().unwrap().to_string();

    // Fetch slots for the lot
    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let slots_json = body_json(resp2).await;
    let slot_id = slots_json["data"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    (lot_id, slot_id)
}

// ═════════════════════════════════════════════════════════════════════════════
// BOOKING WORKFLOW TESTS (closes #63)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_booking_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let start_time = Utc::now() + TimeDelta::hours(1);
    let body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "TEST-001",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["id"].is_string());
    assert_eq!(json["data"]["status"], "confirmed");
}

#[tokio::test]
async fn test_create_booking_slot_unavailable() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let start_time = Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "TEST-001",
    });

    // First booking
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // Second booking for same slot — must conflict
    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "SLOT_UNAVAILABLE");
}

#[tokio::test]
async fn test_create_booking_insufficient_credits() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Enable credits system
    {
        let guard = state.read().await;
        guard
            .db
            .set_setting("credits_enabled", "true")
            .await
            .expect("set credits_enabled");
        guard
            .db
            .set_setting("credits_per_booking", "10")
            .await
            .expect("set credits_per_booking");
    }

    // Register a regular user (credits_balance starts at 0 by default)
    let (user_token, _) =
        register_user_token(state.clone(), "nocredits@example.com", "SecurePass1!").await;

    let start_time = Utc::now() + TimeDelta::hours(1);
    let body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "CRED-001",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INSUFFICIENT_CREDITS");
}

#[tokio::test]
async fn test_cancel_booking() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Create booking
    let start_time = Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "CANCEL-01",
    });

    let booking_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Cancel the booking
    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/bookings/{booking_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn test_cancel_booking_not_own() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Admin creates a booking
    let start_time = Utc::now() + TimeDelta::hours(2);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "NOTOWN-01",
    });

    let booking_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Different user tries to cancel — should get 403
    let (other_token, _) =
        register_user_token(state.clone(), "other@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/bookings/{booking_id}"))
                .header("authorization", format!("Bearer {other_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn test_get_booking_invoice() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Create booking
    let start_time = Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "INV-001",
    });

    let booking_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Fetch invoice
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/bookings/{booking_id}/invoice"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/"),
        "expected text content-type, got: {content_type}"
    );
    let body = String::from_utf8(body_bytes(resp).await).unwrap();
    assert!(!body.is_empty(), "invoice body should not be empty");
}

#[tokio::test]
async fn test_create_guest_booking() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Enable guest bookings
    {
        let guard = state.read().await;
        guard
            .db
            .set_setting("allow_guest_bookings", "true")
            .await
            .expect("set allow_guest_bookings");
    }

    let start_time = Utc::now() + TimeDelta::hours(1);
    let end_time = start_time + TimeDelta::hours(2);
    let guest_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "end_time": end_time,
        "guest_name": "Alice Visitor",
        "guest_email": "alice@visitor.example",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings/guest")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&guest_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["id"].is_string());
    assert!(json["data"]["guest_code"].is_string());
}

#[tokio::test]
async fn test_create_guest_booking_disabled() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Guest bookings are disabled by default — no need to change settings
    let start_time = Utc::now() + TimeDelta::hours(1);
    let end_time = start_time + TimeDelta::hours(2);
    let guest_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "end_time": end_time,
        "guest_name": "Bob Visitor",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings/guest")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&guest_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "GUEST_BOOKINGS_DISABLED");
}

#[tokio::test]
async fn test_booking_checkin() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Create a booking
    let start_time = Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "CHKIN-01",
    });

    let booking_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Check in
    let app = router(state);
    let resp = app
        .oneshot(
            Request::post(format!("/api/v1/bookings/{booking_id}/checkin"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "active");
    assert!(json["data"]["check_in_time"].is_string());
}

#[tokio::test]
async fn test_quick_book() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, _) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let body = serde_json::json!({
        "lot_id": lot_id,
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings/quick")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["id"].is_string());
    assert_eq!(json["data"]["lot_id"].as_str().unwrap(), lot_id);
}

#[tokio::test]
async fn test_quick_book_no_slots_available() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Create lot with only 1 slot, then book it first
    let lot_body = serde_json::json!({
        "name": "Tiny Lot",
        "total_slots": 1,
        "currency": "EUR",
    });
    let lot_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/lots")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&lot_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Quick-book once (takes the only slot)
    {
        let app = router(state.clone());
        let body = serde_json::json!({"lot_id": lot_id});
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings/quick")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Quick-book again — no slots left
    let app = router(state);
    let body = serde_json::json!({"lot_id": lot_id});
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings/quick")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "NO_SLOTS_AVAILABLE");
}

// ═════════════════════════════════════════════════════════════════════════════
// ADMIN TESTS (closes #62 partial)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_admin_list_users() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/users")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"].is_array());
    // Admin user exists
    let users = json["data"].as_array().unwrap();
    assert!(!users.is_empty());
}

#[tokio::test]
async fn test_admin_promote_user() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Register a regular user
    let (_, user_id) =
        register_user_token(state.clone(), "promote@example.com", "SecurePass1!").await;

    // Promote to admin
    let role_body = serde_json::json!({"role": "admin"});
    let app = router(state);
    let resp = app
        .oneshot(
            Request::patch(format!("/api/v1/admin/users/{user_id}/role"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&role_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["role"], "admin");
}

#[tokio::test]
async fn test_admin_list_bookings() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/bookings")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_admin_dashboard_charts() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/dashboard/charts")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    // Chart data should contain expected keys
    assert!(json["data"]["bookings_by_day"].is_array());
    assert!(json["data"]["bookings_by_lot"].is_array());
    assert!(json["data"]["occupancy_by_hour"].is_array());
}

#[tokio::test]
async fn test_non_admin_rejected_from_admin_endpoints() {
    let state = test_state().await;
    let (user_token, _) =
        register_user_token(state.clone(), "nonadmin@example.com", "SecurePass1!").await;

    let app = router(state);

    // Non-admin should get 403 on admin user list
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/users")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "FORBIDDEN");
}

#[tokio::test]
async fn test_non_admin_rejected_from_admin_bookings() {
    let state = test_state().await;
    let (user_token, _) =
        register_user_token(state.clone(), "nonadmin2@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/bookings")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_non_admin_rejected_from_dashboard_charts() {
    let state = test_state().await;
    let (user_token, _) =
        register_user_token(state.clone(), "nonadmin3@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/dashboard/charts")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ═════════════════════════════════════════════════════════════════════════════
// RATE LIMITING TESTS (closes #62 partial)
// ═════════════════════════════════════════════════════════════════════════════

/// Hit the login endpoint 6 times from the same IP (falls back to 127.0.0.1
/// when no ConnectInfo is present).  The rate limiter allows 5 per minute, so
/// the 6th request must return 429.
#[tokio::test]
async fn test_rate_limit_login() {
    let state = test_state().await;

    let bad_body = serde_json::json!({
        "username": "admin",
        "password": "wrong",
    });

    let mut last_status = StatusCode::OK;
    for _ in 0..6 {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&bad_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        last_status = resp.status();
    }

    assert_eq!(
        last_status,
        StatusCode::TOO_MANY_REQUESTS,
        "expected 429 after exceeding login rate limit"
    );
}

/// Register endpoint is limited to 3 per minute — 4th attempt should be 429.
#[tokio::test]
async fn test_rate_limit_register() {
    let state = test_state().await;

    let mut last_status = StatusCode::OK;
    for i in 0..4 {
        let app = router(state.clone());
        let body = serde_json::json!({
            "email": format!("ratelimit{i}@example.com"),
            "password": "SecurePass1!",
            "password_confirmation": "SecurePass1!",
            "name": "RL User",
        });
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        last_status = resp.status();
    }

    assert_eq!(
        last_status,
        StatusCode::TOO_MANY_REQUESTS,
        "expected 429 after exceeding register rate limit"
    );
}
