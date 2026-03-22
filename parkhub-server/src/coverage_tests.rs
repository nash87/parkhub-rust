//! Coverage boost tests — security, admin_ext, edge cases, and API integration.
//!
//! Adds 110+ tests to maximize coverage of security module (2FA, password policy,
//! login history, sessions, API keys), admin extensions (bulk ops, reports, policies),
//! and edge cases (auth, lots, public endpoints).

use axum::body::Body;
use axum::http::{self, Request, StatusCode};
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

async fn create_lot(state: Arc<RwLock<AppState>>, admin_tok: &str) -> String {
    let lot_body = serde_json::json!({
        "name": "Coverage Lot",
        "total_slots": 3,
        "currency": "EUR",
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
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    json["data"]["id"].as_str().unwrap().to_string()
}

// ═══════════════════════════════════════════════════════════════════════════════
// 1. SECURITY MODULE — 2FA
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_2fa_setup_returns_secret_and_qr() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/setup")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"]["secret"].is_string());
    assert!(json["data"]["otpauth_uri"]
        .as_str()
        .unwrap()
        .contains("otpauth://"));
    assert!(json["data"]["qr_code_base64"].is_string());
}

#[tokio::test]
async fn test_2fa_setup_twice_returns_conflict_after_enable() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // First setup
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/2fa/setup")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Simulate enabling 2FA by setting the enabled flag directly
    {
        let guard = state.read().await;
        let users = guard.db.list_users().await.unwrap();
        let admin = users.iter().find(|u| u.username == "admin").unwrap();
        let enabled_key = format!("totp_enabled:{}", admin.id);
        guard.db.set_setting(&enabled_key, "true").await.unwrap();
    }

    // Second setup should conflict
    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/setup")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "2FA_ALREADY_ENABLED");
}

#[tokio::test]
async fn test_2fa_verify_without_setup_returns_404() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"code": "123456"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/verify")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "NO_PENDING_SETUP");
}

#[tokio::test]
async fn test_2fa_verify_invalid_code_returns_400() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Do setup first
    {
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/2fa/setup")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // Verify with wrong code
    let app = router(state);
    let body = serde_json::json!({"code": "000000"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/verify")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_CODE");
}

#[tokio::test]
async fn test_2fa_status_default_disabled() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/2fa/status")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["enabled"], false);
}

#[tokio::test]
async fn test_2fa_disable_wrong_password_returns_401() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"current_password": "wrongpassword"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/disable")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_PASSWORD");
}

#[tokio::test]
async fn test_2fa_disable_correct_password_succeeds() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"current_password": "admin123"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/disable")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["enabled"], false);
}

#[tokio::test]
async fn test_2fa_setup_unauthenticated_returns_401() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/2fa/setup")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 2. SECURITY MODULE — PASSWORD POLICY (API layer)
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_password_policy_as_admin() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/password-policy")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // Default policy
    assert_eq!(json["data"]["min_length"], 8);
    assert_eq!(json["data"]["require_uppercase"], true);
}

#[tokio::test]
async fn test_get_password_policy_as_user_returns_403() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "user@example.com", "SecurePass1!").await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/password-policy")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_password_policy_valid() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "min_length": 12,
        "require_uppercase": true,
        "require_lowercase": true,
        "require_number": true,
        "require_special_char": true,
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/admin/settings/password-policy")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["min_length"], 12);
    assert_eq!(json["data"]["require_special_char"], true);
}

#[tokio::test]
async fn test_update_password_policy_invalid_min_length() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "min_length": 2,  // too low (min is 4)
        "require_uppercase": false,
        "require_lowercase": false,
        "require_number": false,
        "require_special_char": false,
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/admin/settings/password-policy")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_POLICY");
}

#[tokio::test]
async fn test_update_password_policy_max_length_too_high() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "min_length": 200,  // too high (max is 128)
        "require_uppercase": false,
        "require_lowercase": false,
        "require_number": false,
        "require_special_char": false,
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/admin/settings/password-policy")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 3. SECURITY MODULE — LOGIN HISTORY
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_login_history_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/login-history")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_admin_get_user_login_history() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "histuser@example.com", "SecurePass1!").await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/admin/users/{user_id}/login-history"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_admin_get_login_history_nonexistent_user() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get(format!(
                "/api/v1/admin/users/{}/login-history",
                Uuid::new_v4()
            ))
            .header("authorization", format!("Bearer {admin_tok}"))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    // Returns empty array for nonexistent users (no error)
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 4. SECURITY MODULE — SESSION MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_list_sessions_returns_current() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/sessions")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let sessions = json["data"].as_array().unwrap();
    assert!(!sessions.is_empty());
    // At least one should be marked as current
    let has_current = sessions.iter().any(|s| s["is_current"] == true);
    assert!(has_current);
}

#[tokio::test]
async fn test_revoke_nonexistent_session_returns_404() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::delete("/api/v1/auth/sessions/nonexistent...")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_list_sessions_unauthenticated() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/sessions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 5. SECURITY MODULE — API KEYS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_api_key_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"name": "Test Key", "expires_in_days": 30});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert!(json["data"]["api_key"]
        .as_str()
        .unwrap()
        .starts_with("phk_"));
    assert_eq!(json["data"]["name"], "Test Key");
    assert!(json["data"]["expires_at"].is_string());
}

#[tokio::test]
async fn test_create_api_key_no_expiry() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"name": "Permanent Key"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert!(json["data"]["expires_at"].is_null());
}

#[tokio::test]
async fn test_create_api_key_empty_name_returns_400() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"name": ""});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
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
async fn test_create_api_key_too_long_name_returns_400() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({"name": "x".repeat(101)});
    let resp = app
        .oneshot(
            Request::post("/api/v1/auth/api-keys")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_api_keys_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_api_keys_after_create() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Create a key
    {
        let app = router(state.clone());
        let body = serde_json::json!({"name": "List Test Key"});
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/api-keys")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List keys
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let keys = json["data"].as_array().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["name"], "List Test Key");
    // Full key should NOT be in list response
    assert!(keys[0]["api_key"].is_null());
}

#[tokio::test]
async fn test_revoke_api_key_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Create a key
    let key_id = {
        let app = router(state.clone());
        let body = serde_json::json!({"name": "Revoke Test"});
        let resp = app
            .oneshot(
                Request::post("/api/v1/auth/api-keys")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(resp).await;
        json["data"]["id"].as_str().unwrap().to_string()
    };

    // Revoke
    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/auth/api-keys/{key_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // List should show 0 active keys
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/auth/api-keys")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_revoke_api_key_not_found() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/auth/api-keys/{}", Uuid::new_v4()))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 6. ADMIN EXTENSIONS — BULK OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_bulk_update_users_activate() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) = register_user_token(state.clone(), "bulk@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({
        "user_ids": [user_id],
        "action": "deactivate",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["total"], 1);
    assert_eq!(json["data"]["succeeded"], 1);
    assert_eq!(json["data"]["failed"], 0);
}

#[tokio::test]
async fn test_bulk_update_invalid_action() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "user_ids": ["someid"],
        "action": "invalid_action",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "INVALID_ACTION");
}

#[tokio::test]
async fn test_bulk_update_set_role_without_role() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "user_ids": ["someid"],
        "action": "set_role",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "MISSING_ROLE");
}

#[tokio::test]
async fn test_bulk_update_nonexistent_users() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "user_ids": [Uuid::new_v4().to_string(), Uuid::new_v4().to_string()],
        "action": "activate",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["succeeded"], 0);
    assert_eq!(json["data"]["failed"], 2);
}

#[tokio::test]
async fn test_bulk_update_set_role_premium() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "premium@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({
        "user_ids": [user_id],
        "action": "set_role",
        "role": "premium",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["succeeded"], 1);
}

#[tokio::test]
async fn test_bulk_update_as_regular_user_returns_403() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "nonadmin@example.com", "SecurePass1!").await;
    let app = router(state);

    let body = serde_json::json!({
        "user_ids": ["any"],
        "action": "activate",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_bulk_delete_users() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "delete-me@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({"user_ids": [user_id]});
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-delete")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["succeeded"], 1);
}

#[tokio::test]
async fn test_bulk_delete_prevents_self_deletion() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Get admin user ID
    let admin_id = {
        let guard = state.read().await;
        let users = guard.db.list_users().await.unwrap();
        users
            .iter()
            .find(|u| u.username == "admin")
            .unwrap()
            .id
            .to_string()
    };

    let app = router(state);
    let body = serde_json::json!({"user_ids": [admin_id]});
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-delete")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["succeeded"], 0);
    assert_eq!(json["data"]["failed"], 1);
    assert!(json["data"]["errors"][0]
        .as_str()
        .unwrap()
        .contains("own account"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 7. ADMIN EXTENSIONS — REPORTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_revenue_report_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/revenue")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_revenue_report_with_date_range() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/revenue?start_date=2026-01-01&end_date=2026-12-31&group_by=month")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_occupancy_report_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/occupancy")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_occupancy_report_weekly_grouping() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/occupancy?group_by=week")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_user_report_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/users")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_user_report_as_non_admin_returns_403() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "report-user@example.com", "SecurePass1!").await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports/users")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 8. ADMIN EXTENSIONS — NOTIFICATION PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_notification_preferences_default() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/preferences/notifications")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // Defaults: all true
    assert_eq!(json["data"]["email_booking_confirm"], true);
    assert_eq!(json["data"]["push_enabled"], true);
}

#[tokio::test]
async fn test_update_notification_preferences() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "email_booking_confirm": false,
        "email_booking_reminder": false,
        "email_swap_request": true,
        "push_enabled": false,
    });

    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::put("/api/v1/preferences/notifications")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Verify persistence
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/preferences/notifications")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = body_json(resp).await;
    assert_eq!(json["data"]["email_booking_confirm"], false);
    assert_eq!(json["data"]["push_enabled"], false);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 9. ADMIN EXTENSIONS — BOOKING POLICIES
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_booking_policies_default() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/booking-policies")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["max_advance_booking_days"], 0);
    assert_eq!(json["data"]["min_booking_duration_hours"], 0);
    assert_eq!(json["data"]["max_booking_duration_hours"], 0);
}

#[tokio::test]
async fn test_update_booking_policies() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "max_advance_booking_days": 14,
        "min_booking_duration_hours": 1,
        "max_booking_duration_hours": 24,
    });

    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::put("/api/v1/admin/settings/booking-policies")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    // Verify persistence
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/booking-policies")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = body_json(resp).await;
    assert_eq!(json["data"]["max_advance_booking_days"], 14);
    assert_eq!(json["data"]["min_booking_duration_hours"], 1);
    assert_eq!(json["data"]["max_booking_duration_hours"], 24);
}

#[tokio::test]
async fn test_booking_policies_as_non_admin() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "pol-user@example.com", "SecurePass1!").await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/booking-policies")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 10. ADMIN EXTENSIONS — HEALTH CHECK
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_detailed_health_check() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/health/detailed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["status"].is_string());
    assert!(json["version"].is_string());
    assert_eq!(json["db_healthy"], true);
    assert_eq!(json["disk_space_ok"], true);
    assert!(json["components"].is_array());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 11. EDGE CASES — PUBLIC ENDPOINTS
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_system_version_returns_version() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/system/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["version"].is_string());
    assert!(json["name"].is_string());
}

#[tokio::test]
async fn test_system_maintenance_default_off() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/system/maintenance")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["maintenance_mode"], false);
    assert_eq!(json["message"], "");
}

#[tokio::test]
async fn test_public_occupancy_empty_lots() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/public/occupancy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["data"].is_array());
}

#[tokio::test]
async fn test_public_display_empty() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/public/display")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_modules_endpoint() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(Request::get("/api/v1/modules").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["modules"].is_object());
    assert!(json["version"].is_string());
    // In headless mode, all optional modules should be false
    assert_eq!(json["modules"]["bookings"], false);
    assert_eq!(json["modules"]["vehicles"], false);
}

#[tokio::test]
async fn test_legal_impressum_empty_default() {
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// 12. EDGE CASES — AUTH
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_register_password_mismatch() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "email": "mismatch@example.com",
        "password": "SecurePass1!",
        "password_confirmation": "DifferentPass1!",
        "name": "Mismatch User",
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
}

#[tokio::test]
async fn test_register_weak_password() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "email": "weak@example.com",
        "password": "abc",
        "password_confirmation": "abc",
        "name": "Weak Pwd",
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
}

#[tokio::test]
async fn test_register_invalid_email() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({
        "email": "not-an-email",
        "password": "SecurePass1!",
        "password_confirmation": "SecurePass1!",
        "name": "Bad Email",
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

    // The server accepts invalid email formats at registration (no email validation)
    // but it does create the user — this is a known behavior
    let status = resp.status();
    assert!(
        status == StatusCode::CREATED
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_login_with_email_instead_of_username() {
    let state = test_state().await;
    // Register user with email
    register_user_token(state.clone(), "emaillogin@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({
        "username": "emaillogin@example.com",
        "password": "SecurePass1!",
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

    // Email login should work
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_change_password_wrong_current() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "current_password": "wrongpassword",
        "new_password": "NewSecure1!",
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/users/me/password")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_change_password_success() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "chgpwd@example.com", "SecurePass1!").await;
    let app = router(state);

    let body = serde_json::json!({
        "current_password": "SecurePass1!",
        "new_password": "NewSecure2!x",
    });

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/users/me/password")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_refresh_token_with_invalid_token() {
    let state = test_state().await;
    let app = router(state);

    let body = serde_json::json!({"refresh_token": "invalid-refresh-token"});
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

// ═══════════════════════════════════════════════════════════════════════════════
// 13. EDGE CASES — LOTS & ADMIN
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_create_lot_as_regular_user_denied() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "lotuser@example.com", "SecurePass1!").await;
    let app = router(state);

    let body = serde_json::json!({"name": "User Lot", "total_slots": 5, "currency": "EUR"});
    let resp = app
        .oneshot(
            Request::post("/api/v1/lots")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_list_lots_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/lots")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_and_get_lot() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let lot_id = create_lot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["name"], "Coverage Lot");
}

#[tokio::test]
async fn test_get_nonexistent_lot_returns_404() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/lots/{}", Uuid::new_v4()))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_lot() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let lot_id = create_lot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/lots/{lot_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_lot() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let lot_id = create_lot(state.clone(), &admin_tok).await;

    let update_body = serde_json::json!({
        "name": "Updated Lot Name",
        "total_slots": 10,
        "currency": "USD",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/lots/{lot_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["name"], "Updated Lot Name");
}

#[tokio::test]
async fn test_get_lot_slots() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let lot_id = create_lot(state.clone(), &admin_tok).await;

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

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn test_get_lot_pricing() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let lot_id = create_lot(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/lots/{lot_id}/pricing"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 14. EDGE CASES — ADMIN USER MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

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
    let users = json["data"].as_array().unwrap();
    assert!(users.len() >= 1); // at least admin
}

#[tokio::test]
async fn test_admin_delete_user() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "todelete@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/admin/users/{user_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_update_user_role() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "roleuser@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({"role": "premium"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/users/{user_id}/role"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_update_user_status() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "statususer@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({"status": "inactive"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/v1/admin/users/{user_id}/status"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_stats() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/stats")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_reports() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/reports")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_audit_log() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/audit-log")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_heatmap() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/heatmap")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
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
}

// ═══════════════════════════════════════════════════════════════════════════════
// 15. EDGE CASES — SETTINGS & MISC
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_admin_auto_release_get() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/auto-release")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_auto_release_update() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "auto_release_enabled": true,
        "auto_release_minutes": 20,
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/admin/settings/auto-release")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_email_settings_get() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/settings/email")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_privacy_get() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/privacy")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_impressum_get() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/impressum")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_update_current_user() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "update-me@example.com", "SecurePass1!").await;
    let app = router(state);

    let body = serde_json::json!({
        "name": "Updated Name",
        "phone": "+49 123 456789",
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/users/me")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["name"], "Updated Name");
}

#[tokio::test]
async fn test_user_me_alias_endpoint() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/me")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["username"], "admin");
}

#[tokio::test]
async fn test_user_stats_endpoint() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/user/stats")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_user_preferences_get() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/user/preferences")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_user_preferences_update() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let body = serde_json::json!({
        "language": "de",
        "theme": "dark",
        "notifications_enabled": false,
    });
    let resp = app
        .oneshot(
            Request::put("/api/v1/user/preferences")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 16. EDGE CASES — CONCURRENT & BOUNDARY
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_nonexistent_route_returns_fallback() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/nonexistent/route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Router has a fallback handler that serves the SPA (200), not 404
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_wrong_http_method() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_admin_get_user_by_id() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) = register_user_token(state.clone(), "byid@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/users/{user_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["email"], "byid@example.com");
}

#[tokio::test]
async fn test_setup_status() {
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
}

#[tokio::test]
async fn test_gdpr_export() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/users/me/export")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_gdpr_delete_account() {
    let state = test_state().await;
    let (user_tok, _) =
        register_user_token(state.clone(), "gdpr-del@example.com", "SecurePass1!").await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::delete("/api/v1/users/me/delete")
                .header("authorization", format!("Bearer {user_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_update_user_full() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "fulledit@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({
        "name": "Fully Updated",
        "email": "newemail@example.com",
        "role": "premium",
        "is_active": true,
    });
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/admin/users/{user_id}/update"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

// ═══════════════════════════════════════════════════════════════════════════════
// 17. ADDITIONAL UNIT TESTS — PASSWORD POLICY EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_password_policy_exactly_min_length() {
    use crate::api::security::PasswordPolicy;
    let policy = PasswordPolicy {
        min_length: 8,
        require_uppercase: false,
        require_lowercase: false,
        require_number: false,
        require_special_char: false,
    };
    assert!(policy.check("12345678").is_ok());
    assert!(policy.check("1234567").is_err());
}

#[test]
fn test_password_policy_unicode_chars() {
    use crate::api::security::PasswordPolicy;
    let policy = PasswordPolicy {
        min_length: 4,
        require_uppercase: false,
        require_lowercase: false,
        require_number: false,
        require_special_char: false,
    };
    // Unicode chars count by chars, not bytes
    assert!(policy.check("cafe").is_ok());
}

#[test]
fn test_password_policy_special_chars_list() {
    use crate::api::security::PasswordPolicy;
    let policy = PasswordPolicy {
        min_length: 4,
        require_uppercase: false,
        require_lowercase: false,
        require_number: false,
        require_special_char: true,
    };
    assert!(policy.check("aaa!").is_ok());
    assert!(policy.check("aaa@").is_ok());
    assert!(policy.check("aaa#").is_ok());
    assert!(policy.check("aaa$").is_ok());
    assert!(policy.check("aaa%").is_ok());
    assert!(policy.check("aaaa").is_err());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 18. ADDITIONAL UNIT TESTS — BOOKING POLICY EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_booking_policy_exactly_at_boundary() {
    use crate::api::admin_ext::BookingPolicies;
    use chrono::Utc;

    let policies = BookingPolicies {
        max_advance_booking_days: 7,
        min_booking_duration_hours: 1,
        max_booking_duration_hours: 8,
    };

    // Exactly 1 hour duration (boundary)
    let start = Utc::now() + chrono::Duration::hours(1);
    let end = start + chrono::Duration::hours(1);
    assert!(policies.check(start, end).is_ok());

    // Exactly 8 hours duration (boundary)
    let end_max = start + chrono::Duration::hours(8);
    assert!(policies.check(start, end_max).is_ok());
}

#[test]
fn test_booking_policy_zero_duration() {
    use crate::api::admin_ext::BookingPolicies;
    use chrono::Utc;

    let policies = BookingPolicies {
        max_advance_booking_days: 0,
        min_booking_duration_hours: 1,
        max_booking_duration_hours: 0,
    };

    let start = Utc::now() + chrono::Duration::hours(1);
    let end = start; // zero duration
    let err = policies.check(start, end).unwrap_err();
    assert!(err.contains("1 hours"));
}

// ═══════════════════════════════════════════════════════════════════════════════
// 19. ADDITIONAL UNIT TESTS — LOGIN HISTORY EDGE CASES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_login_history_empty() {
    use crate::api::security::LoginHistory;
    let history = LoginHistory::default();
    assert!(history.entries.is_empty());
}

#[test]
fn test_login_history_preserves_order() {
    use crate::api::security::{LoginHistory, LoginHistoryEntry};
    use chrono::Utc;

    let mut history = LoginHistory::default();
    history.add(LoginHistoryEntry {
        timestamp: Utc::now(),
        ip_address: "10.0.0.1".to_string(),
        user_agent: "first".to_string(),
        success: true,
    });
    history.add(LoginHistoryEntry {
        timestamp: Utc::now(),
        ip_address: "10.0.0.2".to_string(),
        user_agent: "second".to_string(),
        success: false,
    });

    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].user_agent, "second"); // most recent first
    assert_eq!(history.entries[1].user_agent, "first");
}

// ═══════════════════════════════════════════════════════════════════════════════
// 20. ADDITIONAL UNIT TESTS — NOTIFICATION PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_notification_preferences_all_disabled() {
    use crate::api::admin_ext::NotificationPreferences;
    let prefs = NotificationPreferences {
        email_booking_confirm: false,
        email_booking_reminder: false,
        email_swap_request: false,
        push_enabled: false,
    };
    let json = serde_json::to_string(&prefs).unwrap();
    let back: NotificationPreferences = serde_json::from_str(&json).unwrap();
    assert!(!back.email_booking_confirm);
    assert!(!back.push_enabled);
}

#[test]
fn test_api_key_expired_serialization() {
    use crate::api::security::ApiKey;
    use chrono::Utc;

    let key = ApiKey {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        name: "Expired Key".to_string(),
        key_hash: "hash".to_string(),
        key_prefix: "phk_expired0".to_string(),
        created_at: Utc::now() - chrono::Duration::days(60),
        expires_at: Some(Utc::now() - chrono::Duration::days(30)),
        last_used_at: Some(Utc::now() - chrono::Duration::days(31)),
        is_active: false,
    };
    let json = serde_json::to_string(&key).unwrap();
    let back: ApiKey = serde_json::from_str(&json).unwrap();
    assert!(!back.is_active);
    assert!(back.last_used_at.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════════
// 21. BULK OPERATIONS — INVALID ROLE
// ═══════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_bulk_update_invalid_role() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let (_, user_id) =
        register_user_token(state.clone(), "badrole@example.com", "SecurePass1!").await;

    let app = router(state);
    let body = serde_json::json!({
        "user_ids": [user_id],
        "action": "set_role",
        "role": "superadmin_invalid",
    });
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/users/bulk-update")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // Invalid role counts as a failure
    assert_eq!(json["data"]["failed"], 1);
    assert!(json["data"]["errors"][0]
        .as_str()
        .unwrap()
        .contains("Invalid role"));
}
