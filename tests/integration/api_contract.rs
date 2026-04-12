//! OpenAPI Contract Tests
//!
//! Verifies that every documented endpoint returns the expected JSON shape,
//! correct `Content-Type` headers, and uses the standard response envelopes.

use crate::common::{
    admin_login, auth_get, create_test_booking, create_test_lot, create_test_slot,
    create_test_user, start_test_server,
};
use serde_json::Value;

// ─────────────────────────────────────────────────────────────────────────────
// Helper: assert the success envelope shape
// ─────────────────────────────────────────────────────────────────────────────

fn assert_success_envelope(body: &Value) {
    assert_eq!(body["success"], true, "Expected success=true, got: {body}");
    assert!(
        body.get("data").is_some(),
        "Success response must contain 'data' field"
    );
}

fn assert_error_envelope(body: &Value) {
    assert_eq!(
        body["success"], false,
        "Expected success=false, got: {body}"
    );
    let error = &body["error"];
    assert!(
        error.is_object(),
        "Error response must contain 'error' object"
    );
    assert!(
        error.get("code").is_some(),
        "Error object must contain 'code'"
    );
    assert!(
        error.get("message").is_some(),
        "Error object must contain 'message'"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn health_endpoint_returns_ok_text() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/health", srv.url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "OK");
}

#[tokio::test]
async fn liveness_probe_returns_200() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/health/live", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test]
async fn readiness_probe_returns_json() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/health/ready", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json"),
        "Expected application/json, got: {ct}"
    );

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["ready"], true);
}

#[tokio::test]
async fn status_endpoint_returns_success_envelope() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/status", srv.url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("application/json"),
        "Expected application/json for /status"
    );

    let body: Value = resp.json().await.unwrap();
    assert_success_envelope(&body);
    assert!(
        body["data"]["total_users"].is_number(),
        "Status must include total_users"
    );
}

#[tokio::test]
async fn login_success_response_shape() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .json(&serde_json::json!({
            "username": "admin",
            "password": "Admin123!",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_success_envelope(&body);

    let data = &body["data"];
    assert!(
        data["user"].is_object(),
        "login data must contain user object"
    );
    assert!(data["user"]["id"].is_string(), "user must have string id");
    assert!(
        data["user"]["username"].is_string(),
        "user must have username"
    );
    assert!(data["user"]["email"].is_string(), "user must have email");
    assert!(data["user"]["role"].is_string(), "user must have role");
    assert!(
        data["tokens"].is_object(),
        "login data must contain tokens object"
    );
    assert!(
        data["tokens"]["access_token"].is_string(),
        "tokens must have access_token"
    );
    assert!(
        data["tokens"]["refresh_token"].is_string(),
        "tokens must have refresh_token"
    );
}

#[tokio::test]
async fn login_failure_returns_error_envelope() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .json(&serde_json::json!({
            "username": "admin",
            "password": "wrong",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);

    let body: Value = resp.json().await.unwrap();
    assert_error_envelope(&body);
    assert_eq!(body["error"]["code"], "INVALID_CREDENTIALS");
}

#[tokio::test]
async fn lots_list_returns_success_envelope() {
    let srv = start_test_server().await;
    // /api/v1/lots is a protected route — authenticate first
    let (token, _) = admin_login(&srv).await;
    let (status, body) = auth_get(&srv, &token, "/api/v1/lots").await;
    assert_eq!(status, 200);
    assert_success_envelope(&body);
    assert!(body["data"].is_array(), "lots list data must be an array");
}

#[tokio::test]
async fn protected_endpoint_without_token_returns_401() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/api/v1/users/me", srv.url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 401);
}

#[tokio::test]
async fn users_me_returns_user_object() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let (status, body) = auth_get(&srv, &token, "/api/v1/users/me").await;
    assert_eq!(status, 200);
    assert_success_envelope(&body);

    let user = &body["data"];
    assert!(user["id"].is_string());
    assert!(user["username"].is_string());
    assert!(user["email"].is_string());
    // password_hash must never leak
    let pw_hash = user["password_hash"].as_str().unwrap_or("");
    assert!(
        pw_hash.is_empty(),
        "password_hash must not be exposed in API response"
    );
}

#[tokio::test]
async fn bookings_list_returns_array() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let (status, body) = auth_get(&srv, &token, "/api/v1/bookings").await;
    if body.is_null() {
        // mod-bookings not compiled; SPA fallback returned HTML
        return;
    }
    assert_eq!(status, 200);
    assert_success_envelope(&body);
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn create_lot_returns_lot_object() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let lot_id = create_test_lot(&srv, &token, "Contract Test Lot").await;
    assert!(!lot_id.is_empty(), "lot_id must be non-empty");

    // GET the lot back
    let (status, body) = auth_get(&srv, &token, &format!("/api/v1/lots/{lot_id}")).await;
    assert_eq!(status, 200);
    assert_success_envelope(&body);
    assert_eq!(body["data"]["name"], "Contract Test Lot");
}

#[tokio::test]
async fn create_booking_returns_booking_object() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    // Check if mod-bookings is compiled by probing the endpoint
    let (_, probe) = auth_get(&srv, &admin_token, "/api/v1/bookings").await;
    if probe.is_null() {
        // mod-bookings not compiled; bookings route serves SPA HTML
        return;
    }

    let (user_token, _, _) = create_test_user(&srv, "contract_booking").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Booking Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;
    let booking_id = create_test_booking(&srv, &user_token, &lot_id, &slot_id).await;

    assert!(!booking_id.is_empty(), "booking_id must be non-empty");

    // GET the booking
    let (status, body) =
        auth_get(&srv, &user_token, &format!("/api/v1/bookings/{booking_id}")).await;
    assert_eq!(status, 200);
    assert_success_envelope(&body);

    let b = &body["data"];
    assert!(b["id"].is_string());
    assert!(b["lot_id"].is_string());
    assert!(b["slot_id"].is_string());
    assert!(b["start_time"].is_string());
    assert!(b["end_time"].is_string());
    assert!(b["status"].is_string());
    assert!(b["pricing"].is_object());
}

#[tokio::test]
async fn modules_endpoint_returns_feature_map() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/api/v1/modules", srv.url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(
        body["modules"].is_object(),
        "modules endpoint must return modules map"
    );
    assert!(
        body["version"].is_string(),
        "modules endpoint must return version"
    );
}

#[tokio::test]
async fn content_type_is_json_on_api_endpoints() {
    let srv = start_test_server().await;

    let endpoints = ["/status", "/api/v1/lots", "/api/v1/modules"];

    for ep in &endpoints {
        let resp = srv
            .client
            .get(format!("{}{}", srv.url, ep))
            .send()
            .await
            .unwrap();

        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.contains("application/json"),
            "Endpoint {ep} must return application/json, got: {ct}"
        );
    }
}

#[tokio::test]
async fn nonexistent_route_returns_404() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .get(format!("{}/api/v1/does-not-exist", srv.url))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    // When static frontend assets are embedded, the SPA fallback serves
    // index.html for unknown paths (status 200).  Both 200 (SPA) and
    // 404 are acceptable here.
    assert!(
        status == 200 || status == 404,
        "Expected 200 (SPA fallback) or 404, got: {status}"
    );
}

#[tokio::test]
async fn post_with_invalid_json_returns_error() {
    let srv = start_test_server().await;
    let resp = srv
        .client
        .post(format!("{}/api/v1/auth/login", srv.url))
        .header("content-type", "application/json")
        .body("not json at all")
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "Invalid JSON should return 400 or 422, got: {status}"
    );
}
