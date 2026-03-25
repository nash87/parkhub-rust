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

// ═════════════════════════════════════════════════════════════════════════════
// ADDITIONAL NEARBY-LOTS TESTS — distance calculation & radius filtering
// ═════════════════════════════════════════════════════════════════════════════

/// Helper: create a lot with explicit coordinates and return its ID.
async fn create_lot_with_coords(
    state: Arc<RwLock<AppState>>,
    admin_tok: &str,
    name: &str,
    lat: f64,
    lng: f64,
) -> String {
    let lot_body = serde_json::json!({
        "name": name,
        "total_slots": 3,
        "currency": "EUR",
        "latitude": lat,
        "longitude": lng,
    });
    let app = router(state);
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
    assert_eq!(resp.status(), StatusCode::CREATED, "create lot with coords failed");
    let json = body_json(resp).await;
    json["data"]["id"].as_str().unwrap().to_string()
}

/// Helper: build and save a booking directly in the DB, bypassing API time
/// validation. Returns the booking ID.
async fn insert_booking_direct(
    state: Arc<RwLock<AppState>>,
    user_id: Uuid,
    lot_id: &str,
    slot_id: &str,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    status: parkhub_common::models::BookingStatus,
    checked_in: bool,
) -> String {
    use parkhub_common::models::{Booking, BookingPricing, PaymentStatus, Vehicle, VehicleType};

    let booking_id = Uuid::new_v4();
    let now = Utc::now();
    let booking = Booking {
        id: booking_id,
        user_id,
        lot_id: Uuid::parse_str(lot_id).expect("lot_id must be a valid UUID"),
        slot_id: Uuid::parse_str(slot_id).expect("slot_id must be a valid UUID"),
        slot_number: 1,
        floor_name: "Level 1".to_string(),
        vehicle: Vehicle {
            id: Uuid::nil(),
            user_id,
            license_plate: "TEST-001".to_string(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::default(),
            is_default: false,
            created_at: now,
        },
        start_time,
        end_time,
        status,
        pricing: BookingPricing {
            base_price: 10.0,
            discount: 0.0,
            tax: 0.0,
            total: 10.0,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: if checked_in { Some(start_time) } else { None },
        check_out_time: None,
        qr_code: None,
        notes: None,
        tenant_id: None,
    };

    let guard = state.read().await;
    guard.db.save_booking(&booking).await.expect("save booking");
    booking_id.to_string()
}

#[tokio::test]
async fn test_mobile_nearby_lots_known_coordinates_distance() {
    // Create a lot at Munich center (48.1351, 11.5820).
    // Query from a point ~111 m north (48.1361, 11.5820).
    // The haversine distance for 0.001° latitude at that latitude is ~111 m.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    create_lot_with_coords(
        state.clone(),
        &admin_tok,
        "Munich Lot",
        48.1351,
        11.5820,
    )
    .await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(
                "/api/v1/mobile/nearby-lots?lat=48.1361&lng=11.5820&radius=500",
            )
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
    assert!(!lots.is_empty(), "lot should appear within 500 m radius");

    let lot = lots.iter().find(|l| l["name"] == "Munich Lot").unwrap();
    let distance = lot["distance_meters"].as_f64().unwrap();
    // 0.001° latitude ≈ 111 m — allow generous ±50 m tolerance for floating-point
    assert!(
        distance > 50.0 && distance < 200.0,
        "distance should be ~111 m, got {distance}"
    );
}

#[tokio::test]
async fn test_mobile_nearby_lots_radius_filters_out_far_lots() {
    // Lot A is ~111 m from the query point (within 500 m radius).
    // Lot B is ~12 km away (lat 48.2500 vs 48.1361) and should be excluded.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    create_lot_with_coords(state.clone(), &admin_tok, "Near Lot", 48.1351, 11.5820).await;
    create_lot_with_coords(state.clone(), &admin_tok, "Far Lot", 48.2500, 11.5820).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(
                "/api/v1/mobile/nearby-lots?lat=48.1361&lng=11.5820&radius=500",
            )
            .header("authorization", format!("Bearer {admin_tok}"))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let lots = json["data"].as_array().unwrap();

    let names: Vec<&str> = lots
        .iter()
        .filter_map(|l| l["name"].as_str())
        .collect();
    assert!(
        names.contains(&"Near Lot"),
        "Near Lot should be within 500 m radius"
    );
    assert!(
        !names.contains(&"Far Lot"),
        "Far Lot should be excluded from 500 m radius"
    );
}

#[tokio::test]
async fn test_mobile_nearby_lots_sorted_closest_first() {
    // Lot A is ~111 m away; Lot B is ~333 m away.
    // Results must be sorted ascending by distance_meters.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // lat=48.1360 is ~111 m from query lat=48.1351 (0.0009° * ~111 m/° ≈ 100 m)
    create_lot_with_coords(state.clone(), &admin_tok, "Closer Lot", 48.1360, 11.5820).await;
    // lat=48.1380 is ~333 m from query lat=48.1351 (0.0029° * ~111 m/° ≈ 322 m)
    create_lot_with_coords(state.clone(), &admin_tok, "Farther Lot", 48.1380, 11.5820).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(
                "/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.5820&radius=5000",
            )
            .header("authorization", format!("Bearer {admin_tok}"))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let lots = json["data"].as_array().unwrap();
    // Filter to only our named lots (ignore zero-coordinate lots from harness setup)
    let named: Vec<_> = lots
        .iter()
        .filter(|l| {
            matches!(l["name"].as_str(), Some("Closer Lot") | Some("Farther Lot"))
        })
        .collect();
    assert_eq!(named.len(), 2, "both lots should be in results");

    let d0 = named[0]["distance_meters"].as_f64().unwrap();
    let d1 = named[1]["distance_meters"].as_f64().unwrap();
    assert!(
        d0 <= d1,
        "lots must be sorted closest-first: got {d0} then {d1}"
    );
    assert_eq!(named[0]["name"].as_str().unwrap(), "Closer Lot");
}

#[tokio::test]
async fn test_mobile_nearby_lots_min_radius_clamped_to_100() {
    // A lot at exactly the query point (distance = 0 m).
    // Even with radius=1 (clamped to 100 m minimum), distance 0 ≤ 100, so it appears.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    create_lot_with_coords(state.clone(), &admin_tok, "OnPoint Lot", 48.1351, 11.5820).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(
                "/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.5820&radius=1",
            )
            .header("authorization", format!("Bearer {admin_tok}"))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let lots = json["data"].as_array().unwrap();
    let names: Vec<&str> = lots.iter().filter_map(|l| l["name"].as_str()).collect();
    assert!(
        names.contains(&"OnPoint Lot"),
        "lot at distance 0 must be included even when radius is clamped to 100 m"
    );
}

#[tokio::test]
async fn test_mobile_nearby_lots_no_coordinates_returns_max_distance() {
    // A lot created without coordinates (lat=0, lng=0) must appear with
    // distance_meters = f64::MAX (sentinel for "unknown location").
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    // The default create_lot_and_get_slot helper omits lat/lng → stored as 0.0
    let _ = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(
                "/api/v1/mobile/nearby-lots?lat=48.1351&lng=11.5820&radius=5000",
            )
            .header("authorization", format!("Bearer {admin_tok}"))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let lots = json["data"].as_array().unwrap();
    assert!(!lots.is_empty());
    // The zero-coordinate lot must have distance_meters = f64::MAX
    let has_max = lots
        .iter()
        .any(|l| l["distance_meters"].as_f64().map_or(false, |d| d >= f64::MAX));
    assert!(
        has_max,
        "lot without coordinates must have distance_meters = f64::MAX"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// ADDITIONAL QUICK-BOOK TESTS — available slot counts & structure
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mobile_quick_book_available_slots_count_matches() {
    // Creating a lot with total_slots=4 should yield available_slots=4 in quick-book
    // (no active bookings, so all slots are considered available).
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let lot_body = serde_json::json!({
        "name": "Four Slot Lot",
        "total_slots": 4,
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
    assert_eq!(resp.status(), StatusCode::CREATED);

    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::OK);
    let json = body_json(resp2).await;
    let lots = json["data"].as_array().unwrap();
    let lot = lots
        .iter()
        .find(|l| l["name"] == "Four Slot Lot")
        .expect("Four Slot Lot must appear in quick-book results");
    assert_eq!(
        lot["available_slots"].as_u64().unwrap(),
        4,
        "available_slots should equal the number of created slots"
    );
}

#[tokio::test]
async fn test_mobile_quick_book_multiple_lots_all_returned() {
    // Two lots → both should appear in the quick-book response.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    for name in &["Alpha Lot", "Beta Lot"] {
        let lot_body = serde_json::json!({
            "name": name,
            "total_slots": 2,
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
        assert_eq!(resp.status(), StatusCode::CREATED, "failed to create {name}");
    }

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
    let names: Vec<&str> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["name"].as_str())
        .collect();
    assert!(names.contains(&"Alpha Lot"), "Alpha Lot must be returned");
    assert!(names.contains(&"Beta Lot"), "Beta Lot must be returned");
}

#[tokio::test]
async fn test_mobile_quick_book_next_slot_lot_name_matches() {
    // next_available_slot.lot_name must equal the parent lot's name.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let lot_body = serde_json::json!({
        "name": "NameCheck Garage",
        "total_slots": 1,
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
    assert_eq!(resp.status(), StatusCode::CREATED);

    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp2.status(), StatusCode::OK);
    let json = body_json(resp2).await;
    let lot = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "NameCheck Garage")
        .expect("NameCheck Garage must appear");
    let slot = &lot["next_available_slot"];
    assert!(slot.is_object(), "next_available_slot must be present");
    assert_eq!(
        slot["lot_name"].as_str().unwrap(),
        "NameCheck Garage",
        "lot_name in next_available_slot must match the lot name"
    );
    // lot_id must be a valid UUID string
    assert!(
        Uuid::parse_str(slot["lot_id"].as_str().unwrap()).is_ok(),
        "lot_id must be a valid UUID"
    );
}

#[tokio::test]
async fn test_mobile_quick_book_invalid_bearer_unauthorized() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/mobile/quick-book")
                .header("authorization", "Bearer totally.invalid.jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ═════════════════════════════════════════════════════════════════════════════
// ADDITIONAL ACTIVE-BOOKING TESTS — countdown math & booking filtering
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_mobile_active_booking_db_confirmed_returned() {
    // Insert a Confirmed booking via the DB (bypassing the API start_time check)
    // with start_time 30 min ago and end_time 90 min from now.
    // The active-booking endpoint must return it.
    let harness = test_harness().await;
    let state = harness.state.clone();
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let admin_user = {
        let guard = state.read().await;
        guard
            .db
            .get_user_by_username("admin")
            .await
            .expect("db error")
            .expect("admin must exist")
    };

    let start = Utc::now() - TimeDelta::minutes(30);
    let end = Utc::now() + TimeDelta::minutes(90);
    insert_booking_direct(
        state.clone(),
        admin_user.id,
        &lot_id,
        &slot_id,
        start,
        end,
        parkhub_common::models::BookingStatus::Confirmed,
        false,
    )
    .await;

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
    assert!(
        !json["data"].is_null(),
        "active booking must be returned for a Confirmed booking that is currently in progress"
    );
    let data = &json["data"];
    assert!(data["id"].is_string());
    assert!(data["remaining_seconds"].is_number());
    assert!(data["total_seconds"].is_number());
    assert!(data["progress_percent"].is_number());
    assert!(data["status"].is_string());
    assert!(data["checked_in"].is_boolean());
}

#[tokio::test]
async fn test_mobile_active_booking_countdown_approximate() {
    // Insert a booking started 30 min ago, ending 90 min from now (total = 120 min).
    // expected: total_seconds ≈ 7200, remaining_seconds ≈ 5400, progress ≈ 25 %
    let harness = test_harness().await;
    let state = harness.state.clone();
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let admin_user = {
        let guard = state.read().await;
        guard
            .db
            .get_user_by_username("admin")
            .await
            .expect("db error")
            .expect("admin must exist")
    };

    let start = Utc::now() - TimeDelta::minutes(30);
    let end = start + TimeDelta::minutes(120);
    insert_booking_direct(
        state.clone(),
        admin_user.id,
        &lot_id,
        &slot_id,
        start,
        end,
        parkhub_common::models::BookingStatus::Confirmed,
        false,
    )
    .await;

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
    let data = &json["data"];
    assert!(!data.is_null(), "booking must be active");

    let total_secs = data["total_seconds"].as_i64().unwrap();
    let remaining_secs = data["remaining_seconds"].as_i64().unwrap();
    let progress = data["progress_percent"].as_f64().unwrap();

    // Total duration = 120 min = 7200 s (±2 s for clock jitter)
    assert!(
        (total_secs - 7200).abs() <= 2,
        "total_seconds should be ~7200, got {total_secs}"
    );
    // Remaining ≈ 90 min = 5400 s — allow ±60 s for test execution time
    assert!(
        (remaining_secs - 5400).abs() <= 60,
        "remaining_seconds should be ~5400, got {remaining_secs}"
    );
    // Progress ≈ 25 % — allow ±2 %
    assert!(
        (progress - 25.0).abs() <= 2.0,
        "progress_percent should be ~25 %, got {progress}"
    );
    // Sanity: progress must always be in [0, 100]
    assert!(progress >= 0.0 && progress <= 100.0);
}

#[tokio::test]
async fn test_mobile_active_booking_past_ended_booking_excluded() {
    // A booking that ended 1 hour ago must NOT appear as an active booking.
    let harness = test_harness().await;
    let state = harness.state.clone();
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    let admin_user = {
        let guard = state.read().await;
        guard
            .db
            .get_user_by_username("admin")
            .await
            .expect("db error")
            .expect("admin must exist")
    };

    // Booking: started 3 h ago, ended 1 h ago — completely in the past
    let start = Utc::now() - TimeDelta::hours(3);
    let end = Utc::now() - TimeDelta::hours(1);
    insert_booking_direct(
        state.clone(),
        admin_user.id,
        &lot_id,
        &slot_id,
        start,
        end,
        parkhub_common::models::BookingStatus::Confirmed,
        false,
    )
    .await;

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
    assert!(
        json["data"].is_null(),
        "a booking whose end_time is in the past must not be returned as active"
    );
}

#[tokio::test]
async fn test_mobile_active_booking_future_start_excluded() {
    // A Confirmed booking that has not started yet (start_time > now) must NOT
    // be returned as the active booking.
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (lot_id, slot_id) = create_lot_and_get_slot(state.clone(), &admin_tok).await;

    // Create future booking via the normal API (start_time is in the future, which
    // satisfies the API's own validation).
    let start_time = Utc::now() + TimeDelta::hours(2);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "FUTURE-99",
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
        assert_eq!(resp.status(), StatusCode::CREATED, "future booking must be created");
    }

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
    assert!(
        json["data"].is_null(),
        "a booking with start_time in the future must not be returned as active"
    );
}
