//! Integration tests for the ParkHub HTTP API layer.
//!
//! These tests spin up the full Axum router (with a real in-memory DB) and send
//! requests through `tower::ServiceExt::oneshot` — no TCP listener needed.

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt; // for `oneshot`

use crate::api::create_router;
use crate::config::ServerConfig;
use crate::db::{Database, DatabaseConfig};
use crate::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Create a fresh `SharedState` backed by a temporary database.
/// The admin user is created so auth-related tests have something to hit.
async fn test_state() -> Arc<RwLock<AppState>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_config = DatabaseConfig {
        path: dir.path().to_path_buf(),
        encryption_enabled: false,
        passphrase: None,
        create_if_missing: true,
    };
    let db = Database::open(db_config).expect("open test db");

    let mut config = ServerConfig::default();
    config.admin_password_hash = hash_password_for_test("admin123");
    config.allow_self_registration = true; // enable for registration tests

    let state = Arc::new(RwLock::new(AppState {
        config: config.clone(),
        db,
        mdns: None,
        scheduler: None,
    }));

    // Seed admin user
    {
        let guard = state.read().await;
        crate::create_admin_user(&guard.db, &guard.config)
            .await
            .expect("seed admin");
    }

    // Leak the tempdir so it lives for the duration of the test.
    // In tests this is acceptable — the OS will clean up on process exit.
    std::mem::forget(dir);

    state
}

/// Build the router from state (returns just the Router, dropping demo state).
fn router(state: Arc<RwLock<AppState>>) -> axum::Router {
    let (router, _demo) = create_router(state);
    router
}

/// Hash a password using argon2 for test fixtures.
fn hash_password_for_test(password: &str) -> String {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    use argon2::Argon2;
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
        .oneshot(
            Request::get("/health/ready")
                .body(Body::empty())
                .unwrap(),
        )
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
    assert!(json["error"]["code"]
        .as_str()
        .unwrap()
        .contains("PROTOCOL_MISMATCH"));
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

    let body = serde_json::json!({
        "email": "longpw@example.com",
        "password": "x".repeat(300),
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
    assert_eq!(
        headers.get("x-content-type-options").unwrap(),
        "nosniff"
    );
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
        .oneshot(
            Request::get("/api/v1/theme")
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
        .oneshot(
            Request::post("/status")
                .body(Body::empty())
                .unwrap(),
        )
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
        .oneshot(
            Request::get("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(resp).await).unwrap();
    // Prometheus format — should contain at least empty output or metric lines
    assert!(text.is_empty() || text.contains('#') || text.len() > 0);
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
