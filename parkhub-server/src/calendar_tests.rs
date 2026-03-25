//! Integration tests for the iCal / calendar sync endpoints.
//!
//! Covers:
//!  - `GET /api/v1/bookings/ical`        — authenticated iCal feed (issue spec)
//!  - `GET /api/v1/calendar/ical`        — authenticated iCal feed alias
//!  - `POST /api/v1/calendar/token`      — generate personal subscription token
//!  - `GET /api/v1/calendar/ical/{token}`— public iCal feed via token

use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

use crate::api::create_router;
use crate::config::ServerConfig;
use crate::db::{Database, DatabaseConfig};
use crate::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

struct TestHarness {
    state: Arc<RwLock<AppState>>,
    _dir: tempfile::TempDir,
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
        admin_password_hash: hash_password("admin123"),
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
    let (r, _demo) = create_router(state);
    r
}

fn hash_password(pw: &str) -> String {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

async fn body_bytes(r: axum::http::Response<Body>) -> Vec<u8> {
    use http_body_util::BodyExt;
    r.into_body()
        .collect()
        .await
        .expect("collect")
        .to_bytes()
        .to_vec()
}

async fn body_json(r: axum::http::Response<Body>) -> serde_json::Value {
    let b = body_bytes(r).await;
    serde_json::from_slice(&b).expect("json")
}

/// Obtain a bearer token for the seeded admin account.
async fn admin_token(state: Arc<RwLock<AppState>>) -> String {
    let body = serde_json::json!({"username":"admin","password":"admin123"});
    let resp = router(state)
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

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GET /api/v1/bookings/ical
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_bookings_ical_requires_auth() {
    let h = test_harness().await;
    let resp = router(h.state)
        .oneshot(
            Request::get("/api/v1/bookings/ical")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_bookings_ical_returns_vcalendar() {
    let h = test_harness().await;
    let tok = admin_token(h.state.clone()).await;

    let resp = router(h.state)
        .oneshot(
            Request::get("/api/v1/bookings/ical")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("text/calendar"),
        "Expected text/calendar, got {ct}"
    );

    let body = String::from_utf8(body_bytes(resp).await).expect("utf8");
    assert!(
        body.starts_with("BEGIN:VCALENDAR"),
        "Missing BEGIN:VCALENDAR"
    );
    assert!(body.contains("VERSION:2.0"), "Missing VERSION:2.0");
    assert!(body.contains("PRODID:-//ParkHub//EN"), "Missing PRODID");
    assert!(body.ends_with("END:VCALENDAR\r\n"), "Missing END:VCALENDAR");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GET /api/v1/calendar/ical (alias)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_calendar_ical_alias_requires_auth() {
    let h = test_harness().await;
    let resp = router(h.state)
        .oneshot(
            Request::get("/api/v1/calendar/ical")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_calendar_ical_alias_returns_vcalendar() {
    let h = test_harness().await;
    let tok = admin_token(h.state.clone()).await;

    let resp = router(h.state)
        .oneshot(
            Request::get("/api/v1/calendar/ical")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = String::from_utf8(body_bytes(resp).await).expect("utf8");
    assert!(body.starts_with("BEGIN:VCALENDAR"));
    assert!(body.ends_with("END:VCALENDAR\r\n"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: POST /api/v1/calendar/token
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_generate_calendar_token_requires_auth() {
    let h = test_harness().await;
    let resp = router(h.state)
        .oneshot(
            Request::post("/api/v1/calendar/token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_generate_calendar_token_returns_token_and_url() {
    let h = test_harness().await;
    let tok = admin_token(h.state.clone()).await;

    let resp = router(h.state)
        .oneshot(
            Request::post("/api/v1/calendar/token")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let token = json["data"]["token"].as_str().unwrap_or("");
    let url = json["data"]["url"].as_str().unwrap_or("");
    assert!(!token.is_empty(), "token should not be empty");
    assert!(
        url.contains("/api/v1/calendar/ical/"),
        "url should contain the ical token path"
    );
    assert!(url.ends_with(token), "url should end with the token");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GET /api/v1/calendar/ical/{token}
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_calendar_ical_by_invalid_token_returns_404() {
    let h = test_harness().await;
    let resp = router(h.state)
        .oneshot(
            Request::get("/api/v1/calendar/ical/nonexistent-token-xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_calendar_ical_by_valid_token_returns_vcalendar() {
    let h = test_harness().await;
    let tok = admin_token(h.state.clone()).await;

    // Generate a subscription token
    let resp = router(h.state.clone())
        .oneshot(
            Request::post("/api/v1/calendar/token")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let cal_token = json["data"]["token"].as_str().unwrap().to_string();

    // Use the token to fetch the public iCal feed (no auth needed)
    let ical_resp = router(h.state)
        .oneshot(
            Request::get(format!("/api/v1/calendar/ical/{cal_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(ical_resp.status(), StatusCode::OK);

    let ct = ical_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("text/calendar"),
        "Expected text/calendar, got {ct}"
    );

    let body = String::from_utf8(body_bytes(ical_resp).await).expect("utf8");
    assert!(
        body.starts_with("BEGIN:VCALENDAR"),
        "Missing BEGIN:VCALENDAR"
    );
    assert!(body.ends_with("END:VCALENDAR\r\n"), "Missing END:VCALENDAR");
}

#[tokio::test]
async fn test_calendar_token_regeneration_revokes_old_token() {
    let h = test_harness().await;
    let tok = admin_token(h.state.clone()).await;

    // Generate first token
    let resp1 = router(h.state.clone())
        .oneshot(
            Request::post("/api/v1/calendar/token")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let j1 = body_json(resp1).await;
    let old_token = j1["data"]["token"].as_str().unwrap().to_string();

    // Generate second token — should revoke the first
    let resp2 = router(h.state.clone())
        .oneshot(
            Request::post("/api/v1/calendar/token")
                .header("authorization", format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let j2 = body_json(resp2).await;
    let new_token = j2["data"]["token"].as_str().unwrap().to_string();

    assert_ne!(old_token, new_token, "New token must differ from old token");

    // Old token should now return 404
    let old_resp = router(h.state)
        .oneshot(
            Request::get(format!("/api/v1/calendar/ical/{old_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        old_resp.status(),
        StatusCode::NOT_FOUND,
        "Revoked token should return 404"
    );
}
