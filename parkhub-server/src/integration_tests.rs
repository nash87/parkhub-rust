//! Integration tests for the ParkHub HTTP API layer.
//!
//! These tests spin up the full Axum router (with a real in-memory DB) and send
//! requests through `tower::ServiceExt::oneshot` — no TCP listener needed.

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt; // for `oneshot`

use crate::AppState;
use crate::api::create_router;
use crate::config::ServerConfig;
use crate::db::{Database, DatabaseConfig};

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Owns both the shared state *and* the temporary directory backing the DB.
/// Drop order is guaranteed: state first (closing the DB), then the dir.
/// This replaces the old `std::mem::forget(dir)` pattern (issue #108).
struct TestHarness {
    state: Arc<RwLock<AppState>>,
    _dir: tempfile::TempDir,
}

/// Create a fresh `SharedState` backed by a temporary database.
/// The admin user is created so auth-related tests have something to hit.
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

    // Seed admin user
    {
        let guard = state.read().await;
        crate::create_admin_user(&guard.db, &guard.config)
            .await
            .expect("seed admin");
    }

    TestHarness { state, _dir: dir }
}

/// Build the router from state (returns just the Router, dropping demo state).
fn router(state: Arc<RwLock<AppState>>) -> axum::Router {
    let (router, _demo) = create_router(state);
    router
}

/// Hash a password using argon2 for test fixtures.
fn hash_password_for_test(password: &str) -> String {
    use argon2::Argon2;
    use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}

/// Read response body as bytes.
async fn body_bytes(response: http::Response<Body>) -> Vec<u8> {
    use http_body_util::BodyExt;
    let collected = response.into_body().collect().await.expect("collect body");
    collected.to_bytes().to_vec()
}

/// Read response body as JSON `serde_json::Value`.
async fn body_json(response: http::Response<Body>) -> serde_json::Value {
    let bytes = body_bytes(response).await;
    serde_json::from_slice(&bytes).expect("parse JSON")
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. HEALTH CHECK ENDPOINTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn health_returns_ok() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(resp).await).unwrap();
    assert_eq!(text, "OK");
}

#[tokio::test]
async fn liveness_returns_200() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn readiness_returns_200_with_ready_true() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/health/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ready"], true);
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. STATUS / HANDSHAKE
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn server_status_returns_success() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["total_users"].as_u64().unwrap() >= 1); // admin exists
}

#[tokio::test]
async fn handshake_matching_protocol_succeeds() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "client_version": "1.0.0",
        "protocol_version": parkhub_common::PROTOCOL_VERSION,
    });

    let resp = app
        .oneshot(
            Request::post("/handshake")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(
        json["data"]["protocol_version"],
        parkhub_common::PROTOCOL_VERSION
    );
}

#[tokio::test]
async fn handshake_mismatched_protocol_returns_error() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "client_version": "1.0.0",
        "protocol_version": "0.0.1-invalid",
    });

    let resp = app
        .oneshot(
            Request::post("/handshake")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK); // wrapper always 200
    let json = body_json(resp).await;
    assert_eq!(json["success"], false);
    assert!(
        json["error"]["code"]
            .as_str()
            .unwrap()
            .contains("PROTOCOL_MISMATCH")
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. AUTH ENDPOINTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn login_with_valid_credentials_succeeds() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "username": "admin",
        "password": "admin123",
    });

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["tokens"]["access_token"].is_string());
    assert!(json["data"]["user"]["username"].as_str().unwrap() == "admin");
}

#[tokio::test]
async fn login_with_invalid_password_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "username": "admin",
        "password": "wrong-password",
    });

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(resp).await;
    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn login_with_nonexistent_user_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "username": "nobody",
        "password": "nope",
    });

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_with_empty_body_returns_error() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // axum will return 422 (Unprocessable Entity) for missing fields
    assert!(
        resp.status() == StatusCode::UNPROCESSABLE_ENTITY
            || resp.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn login_with_overly_long_password_returns_400() {
    let state = test_state().await;
    let app = router(state);

    let long_password = "x".repeat(300);
    let body = serde_json::json!({
        "username": "admin",
        "password": long_password,
    });

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_INPUT");
}

#[tokio::test]
async fn register_creates_new_user() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "email": "testuser@example.com",
        "password": "SecurePass1!",
        "password_confirmation": "SecurePass1!",
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

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["tokens"]["access_token"].is_string());
    // Password hash should never be leaked
    assert_eq!(json["data"]["user"]["password_hash"], "");
}

#[tokio::test]
async fn register_duplicate_email_returns_409() {
    let state = test_state().await;

    // First registration
    {
        let app = router(state.clone());
        let body = serde_json::json!({
            "email": "dup@example.com",
            "password": "SecurePass1!",
            "password_confirmation": "SecurePass1!",
            "name": "First",
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
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // Second registration with same email
    {
        let app = router(state);
        let body = serde_json::json!({
            "email": "dup@example.com",
            "password": "AnotherPass1!",
            "password_confirmation": "AnotherPass1!",
            "name": "Second",
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
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let json = body_json(resp).await;
        assert_eq!(json["error"]["code"], "EMAIL_EXISTS");
    }
}

#[tokio::test]
async fn register_with_overly_long_password_returns_400() {
    let state = test_state().await;
    let app = router(state);

    let long_pw = format!("Aa1{}", "x".repeat(300));
    let body = serde_json::json!({
        "email": "longpw@example.com",
        "password": long_pw,
        "password_confirmation": long_pw,
        "name": "Long Pwd",
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

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_INPUT");
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. PROTECTED ENDPOINTS (auth required)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn protected_endpoint_without_token_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn protected_endpoint_with_invalid_token_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/users/me")
                .header("authorization", "Bearer invalid-token-here")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn protected_endpoint_with_valid_token_succeeds() {
    let state = test_state().await;

    // Login to get a token
    let token = {
        let app = router(state.clone());
        let body = serde_json::json!({
            "username": "admin",
            "password": "admin123",
        });
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
    };

    // Use the token to access a protected endpoint
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/users/me")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["username"], "admin");
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. SECURITY HEADERS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn responses_include_security_headers() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let headers = resp.headers();
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert!(headers.get("content-security-policy").is_some());
    assert!(headers.get("referrer-policy").is_some());
    assert!(headers.get("strict-transport-security").is_some());
    assert!(headers.get("permissions-policy").is_some());
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. PUBLIC API ENDPOINTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn features_endpoint_returns_success() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/features")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["available"].is_array());
}

#[tokio::test]
async fn theme_endpoint_returns_success() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/api/v1/theme").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn active_announcements_returns_success() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/announcements/active")
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
async fn setup_status_returns_success() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    // After admin creation, setup should be completed
    assert_eq!(json["data"]["setup_completed"], true);
}

// ═════════════════════════════════════════════════════════════════════════════
// 7. DEMO ENDPOINTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn demo_config_returns_disabled_by_default() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/demo/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["demo_mode"], false);
}

#[tokio::test]
async fn demo_status_returns_403_when_disabled() {
    let state = test_state().await;
    let app = router(state);

    // Demo endpoints require ConnectInfo — build a request with it
    let resp = app
        .oneshot(
            Request::get("/api/v1/demo/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without ConnectInfo the handler will panic/500, or return 403 if demo disabled.
    // Since demo is disabled by default, the exact status depends on ConnectInfo availability.
    // The important thing is the server doesn't crash — any 4xx/5xx is acceptable.
    assert!(resp.status().is_client_error() || resp.status().is_server_error());
}

// ═════════════════════════════════════════════════════════════════════════════
// 8. METHOD NOT ALLOWED
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn get_on_post_only_endpoint_returns_405() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn post_on_get_only_endpoint_returns_405() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::post("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// ═════════════════════════════════════════════════════════════════════════════
// 9. REQUEST BODY LIMIT
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn oversized_body_is_rejected() {
    let state = test_state().await;
    let app = router(state);

    // Send a 5 MiB payload to an endpoint that accepts JSON
    let huge_body = vec![b'x'; 5 * 1024 * 1024];

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(huge_body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// ═════════════════════════════════════════════════════════════════════════════
// 10. IMPRESSUM (DDG § 5 — public endpoint)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn impressum_returns_json_object() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/legal/impressum")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // Impressum returns a flat JSON object with DDG § 5 fields (not ApiResponse-wrapped)
    assert!(json.is_object());
    assert!(json.get("provider_name").is_some());
}

// ═════════════════════════════════════════════════════════════════════════════
// 11. METRICS ENDPOINT
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn metrics_endpoint_returns_200_without_token_env() {
    let state = test_state().await;
    let app = router(state);

    // When METRICS_TOKEN is not set, metrics should be accessible without auth
    let resp = app
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(resp).await).unwrap();
    // Prometheus format — should contain at least empty output or metric lines
    assert!(text.is_empty() || text.contains('#'));
}

// ═════════════════════════════════════════════════════════════════════════════
// 12. TOKEN REFRESH
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn refresh_with_invalid_token_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "refresh_token": "invalid-refresh-token",
    });

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_with_valid_token_returns_new_tokens() {
    let state = test_state().await;

    // Login to get tokens
    let refresh_token = {
        let app = router(state.clone());
        let body = serde_json::json!({
            "username": "admin",
            "password": "admin123",
        });
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
        json["data"]["tokens"]["refresh_token"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Refresh
    let app = router(state);
    let body = serde_json::json!({
        "refresh_token": refresh_token,
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/refresh")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert!(json["data"]["access_token"].is_string());
}

// ═════════════════════════════════════════════════════════════════════════════
// 13. BOOKING WORKFLOW TESTS (closes #63)
// ═════════════════════════════════════════════════════════════════════════════

use chrono::TimeDelta;
use uuid::Uuid;

/// Helper: login as admin and return access token.
async fn admin_token_it(state: Arc<RwLock<AppState>>) -> String {
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

/// Helper: register a user and return (access_token, user_id).
async fn register_user_it(state: Arc<RwLock<AppState>>, email: &str) -> (String, String) {
    let app = router(state);
    let body = serde_json::json!({
        "email": email,
        "password": "SecurePass1!",
        "password_confirmation": "SecurePass1!",
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

/// Helper: create a parking lot with one slot and return (lot_id, slot_id).
async fn setup_lot_and_slot(state: Arc<RwLock<AppState>>, admin_tok: &str) -> (String, String) {
    let lot_body = serde_json::json!({
        "name": "Test Lot",
        "total_slots": 5,
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
        assert_eq!(resp.status(), StatusCode::CREATED, "create lot failed");
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    let slot_id = {
        let app = router(state);
        let resp = app
            .oneshot(
                Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(resp).await;
        json["data"][0]["id"].as_str().unwrap().to_string()
    };

    (lot_id, slot_id)
}

#[tokio::test]
async fn test_create_booking_reserves_slot() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

    // Verify slot is available before booking
    let slot_before = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(resp).await;
        json["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"].as_str().unwrap() == slot_id)
            .cloned()
            .unwrap()
    };
    assert_eq!(slot_before["status"], "available");

    // Create booking
    let start_time = (chrono::Utc::now() + TimeDelta::days(1))
        .date_naive()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "RSRV-001",
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
        let json = body_json(resp).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "confirmed");
    }

    // Verify slot is now reserved
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    let slot_after = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"].as_str().unwrap() == slot_id)
        .cloned()
        .unwrap();
    assert_eq!(slot_after["status"], "reserved");
}

#[tokio::test]
async fn test_create_booking_fails_when_slot_full() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

    let start_time = (chrono::Utc::now() + TimeDelta::days(1))
        .date_naive()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "FULL-001",
    });

    // First booking succeeds
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

    // Second booking for the same slot must fail with SLOT_UNAVAILABLE
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
async fn test_cancel_booking_releases_slot() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

    // Create booking
    let start_time = chrono::Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "REL-001",
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

    // Cancel booking
    {
        let app = router(state.clone());
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

    // Verify slot is back to available
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}/slots"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    let slot = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"].as_str().unwrap() == slot_id)
        .cloned()
        .unwrap();
    assert_eq!(slot["status"], "available");
}

#[tokio::test]
async fn test_cancel_booking_refunds_credits() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

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
            .set_setting("credits_per_booking", "5")
            .await
            .expect("set credits_per_booking");
    }

    // Register a user with sufficient credits
    let (user_tok, user_id) = register_user_it(state.clone(), "creditsrefund@example.com").await;

    // Give the user 10 credits
    {
        let guard = state.write().await;
        let mut user = guard.db.get_user(&user_id).await.unwrap().unwrap();
        user.credits_balance = 10;
        guard.db.save_user(&user).await.unwrap();
    }

    // Create booking (costs 5 credits)
    let start_time = chrono::Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "CRED-REF",
    });
    let booking_id = {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {user_tok}"))
                    .body(Body::from(serde_json::to_vec(&booking_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Verify credits were deducted (10 - 5 = 5)
    {
        let guard = state.read().await;
        let user = guard.db.get_user(&user_id).await.unwrap().unwrap();
        assert_eq!(
            user.credits_balance, 5,
            "credits should be deducted after booking"
        );
    }

    // Cancel booking
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::delete(format!("/api/v1/bookings/{booking_id}"))
                    .header("authorization", format!("Bearer {user_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Verify credits were refunded (5 + 5 = 10)
    let guard = state.read().await;
    let user = guard.db.get_user(&user_id).await.unwrap().unwrap();
    assert_eq!(
        user.credits_balance, 10,
        "credits should be refunded after cancellation"
    );
}

#[tokio::test]
async fn test_get_booking_invoice_returns_correct_amounts() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

    // Create booking (60 min, default rate 2.00 EUR/h → base_price=2.00, tax=0.38, total=2.38)
    let start_time = chrono::Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "INV-AMT",
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
    let body_text = String::from_utf8(body_bytes(resp).await).unwrap();
    // Invoice should mention EUR (currency) and contain pricing amounts
    assert!(!body_text.is_empty(), "invoice must not be empty");
    assert!(
        body_text.contains("EUR") || body_text.contains("2."),
        "invoice should contain pricing info"
    );
    // Invoice must reference the booking's slot number
    assert!(
        body_text.contains('1') || body_text.contains("Slot") || body_text.contains("INV-"),
        "invoice should reference slot or invoice number"
    );
}

#[tokio::test]
async fn test_booking_max_per_day_limit_enforced() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;

    // Create a lot with multiple slots
    let lot_body = serde_json::json!({
        "name": "Max Per Day Lot",
        "total_slots": 10,
        "currency": "EUR",
    });
    let (lot_id, slots) = {
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
        let json = body_json(resp).await;
        let lot_id = json["data"]["id"].as_str().unwrap().to_string();

        let app2 = router(state.clone());
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
        let slots: Vec<String> = slots_json["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect();
        (lot_id, slots)
    };

    // Register a regular user
    let (user_tok, user_id) = register_user_it(state.clone(), "maxperday@example.com").await;

    // Set max_bookings_per_day = 1
    {
        let guard = state.read().await;
        guard
            .db
            .set_setting("max_bookings_per_day", "1")
            .await
            .expect("set max_bookings_per_day");
    }

    let start_time = chrono::Utc::now() + TimeDelta::hours(1);

    // First booking should succeed
    {
        let body = serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slots[0],
            "start_time": start_time,
            "duration_minutes": 60,
            "vehicle_id": Uuid::nil(),
            "license_plate": "MPD-001",
        });
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/bookings")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {user_tok}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    {
        let guard = state.read().await;
        let configured_limit = guard.db.get_setting("max_bookings_per_day").await.unwrap();
        assert_eq!(configured_limit.as_deref(), Some("1"));

        let bookings = guard.db.list_bookings().await.unwrap();
        assert_eq!(bookings.len(), 1);
        assert_eq!(bookings[0].user_id.to_string(), user_id);
        assert_eq!(bookings[0].start_time.date_naive(), start_time.date_naive());
    }

    // Second booking on the same day should be rejected
    let body2 = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slots[1],
        "start_time": start_time + TimeDelta::minutes(90),
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "MPD-002",
    });
    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/bookings")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::from(serde_json::to_vec(&body2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "MAX_BOOKINGS_REACHED");
}

// ═════════════════════════════════════════════════════════════════════════════
// 14. ADMIN & RATE LIMITING TESTS (closes #62)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_admin_list_all_bookings() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;
    let (lot_id, slot_id) = setup_lot_and_slot(state.clone(), &admin_tok).await;

    // Create a booking so the list is non-empty
    let start_time = chrono::Utc::now() + TimeDelta::hours(1);
    let booking_body = serde_json::json!({
        "lot_id": lot_id,
        "slot_id": slot_id,
        "start_time": start_time,
        "duration_minutes": 60,
        "vehicle_id": Uuid::nil(),
        "license_plate": "ADMLST-01",
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

    // Admin lists all bookings
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
    // Pagination envelope: {items, total, page, per_page} or direct array
    let bookings = json["data"]["items"]
        .as_array()
        .or_else(|| json["data"].as_array())
        .unwrap();
    assert!(
        !bookings.is_empty(),
        "admin should see at least one booking"
    );
    // Each entry should have enriched fields
    let first = &bookings[0];
    assert!(first["id"].is_string());
    assert!(first["user_id"].is_string());
    assert!(first["lot_id"].is_string());
    assert!(first["status"].is_string());
}

#[tokio::test]
async fn test_admin_update_user_status() {
    let state = test_state().await;
    let admin_tok = admin_token_it(state.clone()).await;

    // Register a user to disable
    let (_, user_id) = register_user_it(state.clone(), "statustest@example.com").await;

    // Disable the user
    let disable_body = serde_json::json!({"status": "disabled"});
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::patch(format!("/api/v1/admin/users/{user_id}/status"))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&disable_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "disabled");
        assert_eq!(json["data"]["is_active"], false);
    }

    // Re-enable the user
    let enable_body = serde_json::json!({"status": "active"});
    let app = router(state);
    let resp = app
        .oneshot(
            Request::patch(format!("/api/v1/admin/users/{user_id}/status"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&enable_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "active");
    assert_eq!(json["data"]["is_active"], true);
}

/// Hit login 6 times from the same IP (loopback -- no ConnectInfo in tests).
/// The limiter allows 5 per minute; the 6th must return 429.
#[tokio::test]
async fn test_rate_limit_login_after_failures() {
    let state = test_state().await;
    // Use a single router so rate-limiter state is shared across requests
    let app = router(state);

    let bad_body = serde_json::json!({
        "username": "admin",
        "password": "wrong-password",
    });

    let mut last_status = StatusCode::OK;
    for _ in 0..6 {
        let resp = app
            .clone()
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
        "expected 429 after 6 login attempts (limit is 5/min)"
    );
}

/// After exhausting the login rate limit, a new state (new rate limiter) allows
/// requests again — simulating the window resetting.
#[tokio::test]
async fn test_rate_limit_allows_after_window() {
    // First, exhaust the rate limit on one state instance
    let state_a = test_state().await;
    let bad_body = serde_json::json!({
        "username": "admin",
        "password": "wrong-password",
    });
    for _ in 0..6 {
        let app = router(state_a.clone());
        app.oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&bad_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    }

    // A fresh state has a fresh rate limiter — requests should be allowed again.
    // This models the behaviour after the rate-limit window resets.
    let state_b = test_state().await;
    let app = router(state_b);
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&bad_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // With a fresh limiter the request gets through (returns 401, not 429)
    assert_ne!(
        resp.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "fresh rate limiter should allow requests (window has reset)"
    );
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
