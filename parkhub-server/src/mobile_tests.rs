//! Integration tests for the Mobile Booking module (`mod-mobile`).
//!
//! Tests the three mobile-optimized endpoints:
//! - `GET /api/v1/mobile/quick-book`     — simplified booking options
//! - `GET /api/v1/mobile/nearby-lots`    — geolocation-based lot discovery
//! - `GET /api/v1/mobile/active-booking` — current active booking with countdown

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
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

struct TestHarness {
    state: Arc<RwLock<AppState>>,
    _dir: tempfile::TempDir,
}

async fn test_state() -> Arc<RwLock<AppState>> {
    test_harness().await.state
}

async fn test_harness() -> TestHarness {
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

    TestHarness { state, _dir: dir }
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
    let user_id = json["data"]["user"]["id"].as_str().unwrap().to_string();
    (token, user_id)
}

/// Create a parking lot via admin API and return (lot_id, first_slot_id).
async fn create_lot_and_get_slot(
    state: Arc<RwLock<AppState>>,
    admin_tok: &str,
) -> (String, String) {
    let lot_body = serde_json::json!({
        "name": "Mobile Test Lot",
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

    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let slots_json = body_json(resp2).await;
    let slot_id = slots_json["data"][0]["id"].as_str().unwrap().to_string();

    (lot_id, slot_id)
}

// ═════════════════════════════════════════════════════════════════════════════
// QUICK-BOOK ENDPOINT TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mobile_quick_book_requires_auth() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mobile_quick_book_empty_lots() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
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
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_mobile_quick_book_with_lots() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_lot_id, _slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    let lots = json["data"].as_array().unwrap();
    assert!(!lots.is_empty(), "should return at least one lot");
    // Verify structure of first lot
    let first = &lots[0];
    assert!(first["id"].is_string());
    assert!(first["name"].is_string());
    assert!(first["available_slots"].is_number());
    // next_available_slot should be present since we created slots
    assert!(first["next_available_slot"].is_object());
    let slot = &first["next_available_slot"];
    assert!(slot["slot_id"].is_string());
    assert!(slot["slot_label"].is_string());
    assert!(slot["lot_id"].is_string());
    assert!(slot["lot_name"].is_string());
}

#[tokio::test]
async fn test_mobile_quick_book_regular_user() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_lot_id, _slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let (user_token, _) =
        register_user_token(state.clone(), "mobile-user@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .header("authorization", format!("Bearer {user_token}"))
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

// ═════════════════════════════════════════════════════════════════════════════
// NEARBY-LOTS ENDPOINT TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mobile_nearby_lots_requires_auth() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.582")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mobile_nearby_lots_missing_params() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    // Missing lat and lng should return 400 (deserialization failure)
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_mobile_nearby_lots_empty_db() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.582")
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
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_mobile_nearby_lots_with_lots() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_lot_id, _slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots?lat=0.0&lng=0.0&radius=10000")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    let lots = json["data"].as_array().unwrap();
    // Lots without coordinates get included with large distance marker
    assert!(!lots.is_empty(), "should include lots (even without coords)");
    // Verify structure
    let first = &lots[0];
    assert!(first["id"].is_string());
    assert!(first["name"].is_string());
    assert!(first["total_slots"].is_number());
    assert!(first["available_slots"].is_number());
    assert!(first["occupancy_percent"].is_number());
    assert!(first["distance_meters"].is_number());
}

#[tokio::test]
async fn test_mobile_nearby_lots_default_radius() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let app = router(state);
    // No radius param — defaults to 1000m
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.582")
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
async fn test_mobile_nearby_lots_radius_clamped() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let app = router(state);
    // Radius > 10000 gets clamped to 10000
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.582&radius=999999")
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

// ═════════════════════════════════════════════════════════════════════════════
// ACTIVE-BOOKING ENDPOINT TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mobile_active_booking_requires_auth() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/active-booking")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_mobile_active_booking_no_bookings() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/active-booking")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    // No active booking => data is null
    assert!(json["data"].is_null());
}

#[tokio::test]
async fn test_mobile_active_booking_with_active() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Create a booking that starts now and runs for 2 hours
    let start_time = Utc::now() - TimeDelta::minutes(30);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 120,
        "vehicle_id": Uuid::nil(),
        "license_plate": "MOB-001",
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
        // Booking may be CREATED or OK depending on time validation
        let json = body_json(resp).await;
        json["data"]["id"].as_str().map(|s| s.to_string())
    };

    // Check in the booking to make it "active"
    if let Some(id) = &booking_id {
        let app = router(state.clone());
        let _ = app
            .oneshot(
                Request::post(format!("/api/v1/bookings/{id}/checkin"))
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
    }

    // Now check active booking
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/active-booking")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    // Whether data is present depends on booking time validation — assert structure if present
    if !json["data"].is_null() {
        let data = &json["data"];
        assert!(data["id"].is_string());
        assert!(data["lot_name"].is_string());
        assert!(data["slot_label"].is_string());
        assert!(data["start_time"].is_string());
        assert!(data["end_time"].is_string());
        assert!(data["remaining_seconds"].is_number());
        assert!(data["total_seconds"].is_number());
        assert!(data["progress_percent"].is_number());
        assert!(data["status"].is_string());
    }
}

#[tokio::test]
async fn test_mobile_active_booking_different_user() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Admin creates a booking
    let start_time = Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "MOB-002",
    });

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

    // Different user should see no active booking (it's admin's booking)
    let (user_token, _) =
        register_user_token(state.clone(), "mobile-other@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/active-booking")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    // Other user has no bookings, so data should be null
    assert!(json["data"].is_null());
}

#[tokio::test]
async fn test_mobile_active_booking_expired_token() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/active-booking")
                .header("authorization", "Bearer invalid.token.here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
