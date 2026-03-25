//! Integration tests for the Webhooks V2 module (`mod-webhooks-v2`).
//!
//! Tests the webhook management endpoints under `/api/v1/admin/webhooks-v2`:
//! - `GET    /api/v1/admin/webhooks-v2`              — list subscriptions
//! - `POST   /api/v1/admin/webhooks-v2`              — create subscription
//! - `PUT    /api/v1/admin/webhooks-v2/{id}`          — update subscription
//! - `DELETE /api/v1/admin/webhooks-v2/{id}`          — delete subscription
//! - `POST   /api/v1/admin/webhooks-v2/{id}/test`     — send test event
//! - `GET    /api/v1/admin/webhooks-v2/{id}/deliveries` — delivery log

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

/// Create a webhook via admin API and return its id.
async fn create_webhook(state: Arc<RwLock<AppState>>, admin_tok: &str) -> String {
    let body = serde_json::json!({
        "url": "https://example.com/webhook",
        "events": ["booking.created"],
        "description": "Test webhook",
    });
    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED, "create webhook failed");
    let json = body_json(resp).await;
    json["data"]["id"].as_str().unwrap().to_string()
}

// ═════════════════════════════════════════════════════════════════════════════
// AUTH & ACCESS CONTROL TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_list_requires_auth() {
    let state = test_state().await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_webhooks_v2_list_requires_admin() {
    let state = test_state().await;
    let (user_token, _) =
        register_user_token(state.clone(), "nonadmin-wh@example.com", "SecurePass1!").await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_webhooks_v2_create_requires_admin() {
    let state = test_state().await;
    let (user_token, _) =
        register_user_token(state.clone(), "nonadmin-wh2@example.com", "SecurePass1!").await;

    let body = serde_json::json!({
        "url": "https://example.com/hook",
        "events": ["booking.created"],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {user_token}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

// ═════════════════════════════════════════════════════════════════════════════
// CRUD TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_list_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let app = router(state);

    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
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
async fn test_webhooks_v2_create_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "https://example.com/webhook",
        "events": ["booking.created", "lot.full"],
        "description": "My test webhook",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
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
    let data = &json["data"];
    assert!(data["id"].is_string());
    assert_eq!(data["url"], "https://example.com/webhook");
    assert!(data["secret"].as_str().unwrap().starts_with("whsec_"));
    assert_eq!(data["active"], true, "webhook should be active by default");
    assert_eq!(data["events"].as_array().unwrap().len(), 2);
    assert_eq!(data["description"], "My test webhook");
    assert!(data["created_at"].is_string());
    assert!(data["updated_at"].is_string());
}

#[tokio::test]
async fn test_webhooks_v2_create_and_list() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let _wh_id = create_webhook(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    let webhooks = json["data"].as_array().unwrap();
    assert_eq!(webhooks.len(), 1);
    assert_eq!(webhooks[0]["url"], "https://example.com/webhook");
}

#[tokio::test]
async fn test_webhooks_v2_update_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let wh_id = create_webhook(state.clone(), &admin_tok).await;

    let update_body = serde_json::json!({
        "url": "https://updated.example.com/hook",
        "active": false,
        "description": "Updated description",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/admin/webhooks-v2/{wh_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["url"], "https://updated.example.com/hook");
    assert_eq!(json["data"]["active"], false);
    assert_eq!(json["data"]["description"], "Updated description");
}

#[tokio::test]
async fn test_webhooks_v2_update_not_found() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let fake_id = Uuid::new_v4();

    let update_body = serde_json::json!({"active": false});

    let app = router(state);
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/admin/webhooks-v2/{fake_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_webhooks_v2_delete_success() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let wh_id = create_webhook(state.clone(), &admin_tok).await;

    let app = router(state.clone());
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/admin/webhooks-v2/{wh_id}"))
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);

    // Verify it's gone
    let app2 = router(state);
    let resp2 = app2
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json2 = body_json(resp2).await;
    assert_eq!(json2["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_webhooks_v2_delete_not_found() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let fake_id = Uuid::new_v4();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::delete(format!("/api/v1/admin/webhooks-v2/{fake_id}"))
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

// ═════════════════════════════════════════════════════════════════════════════
// VALIDATION TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_create_invalid_url() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "not a url",
        "events": ["booking.created"],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_webhooks_v2_create_empty_events() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "https://example.com/hook",
        "events": [],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("event"),
        "error message should mention events"
    );
}

#[tokio::test]
async fn test_webhooks_v2_create_unknown_event() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "https://example.com/hook",
        "events": ["booking.created", "unknown.event"],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown.event"),
        "error should mention the invalid event type"
    );
}

#[tokio::test]
async fn test_webhooks_v2_update_invalid_url() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let wh_id = create_webhook(state.clone(), &admin_tok).await;

    let update_body = serde_json::json!({
        "url": "ftp://invalid-scheme.com",
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/admin/webhooks-v2/{wh_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_webhooks_v2_update_invalid_events() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let wh_id = create_webhook(state.clone(), &admin_tok).await;

    let update_body = serde_json::json!({
        "events": ["bogus.event"],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::put(format!("/api/v1/admin/webhooks-v2/{wh_id}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp).await;
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
}

// ═════════════════════════════════════════════════════════════════════════════
// DELIVERIES TESTS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_deliveries_not_found() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let fake_id = Uuid::new_v4();

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/admin/webhooks-v2/{fake_id}/deliveries"))
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
async fn test_webhooks_v2_deliveries_empty() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;
    let wh_id = create_webhook(state.clone(), &admin_tok).await;

    let app = router(state);
    let resp = app
        .oneshot(
            Request::get(format!("/api/v1/admin/webhooks-v2/{wh_id}/deliveries"))
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

// ═════════════════════════════════════════════════════════════════════════════
// LOCALHOST HTTP ALLOWED IN DEBUG MODE
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_create_localhost_http_debug() {
    // In debug mode, http://localhost should be accepted
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "http://localhost:8080/webhook",
        "events": ["payment.completed"],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // In debug_assertions mode, http://localhost is allowed
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["url"], "http://localhost:8080/webhook");
}

// ═════════════════════════════════════════════════════════════════════════════
// MULTIPLE WEBHOOKS
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhooks_v2_create_multiple_and_list() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    // Create 3 webhooks
    for i in 1..=3 {
        let body = serde_json::json!({
            "url": format!("https://example.com/hook/{i}"),
            "events": ["booking.created"],
            "description": format!("Hook {i}"),
        });
        let app = router(state.clone());
        let resp = app
            .oneshot(
                Request::post("/api/v1/admin/webhooks-v2")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {admin_tok}"))
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // List all
    let app = router(state);
    let resp = app
        .oneshot(
            Request::get("/api/v1/admin/webhooks-v2")
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
async fn test_webhooks_v2_create_with_all_events() {
    let state = test_state().await;
    let admin_tok = admin_token(state.clone()).await;

    let body = serde_json::json!({
        "url": "https://example.com/all-events",
        "events": [
            "booking.created",
            "booking.cancelled",
            "user.registered",
            "lot.full",
            "payment.completed"
        ],
    });

    let app = router(state);
    let resp = app
        .oneshot(
            Request::post("/api/v1/admin/webhooks-v2")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {admin_tok}"))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    assert_eq!(json["data"]["events"].as_array().unwrap().len(), 5);
}
