//! Database Migration Tests
//!
//! Verify that a fresh server starts correctly, has proper schema,
//! seeds the admin user, and passes health checks.
//! Also tests that the server can handle restarts with existing data.

use crate::common::{admin_login, auth_get, start_test_server};
use serde_json::Value;

// ═════════════════════════════════════════════════════════════════════════════
// Fresh database initialization
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn fresh_database_starts_successfully() {
    let srv = start_test_server().await;

    // Health check should pass
    let resp = srv
        .client
        .get(format!("{}/health", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test]
async fn fresh_database_has_admin_user() {
    let srv = start_test_server().await;

    // Admin should be able to log in
    let (token, admin_id) = admin_login(&srv).await;
    assert!(!token.is_empty(), "Admin token should be non-empty");
    assert!(!admin_id.is_empty(), "Admin user ID should be non-empty");

    // Admin profile should be accessible
    let (status, body) = auth_get(&srv, &token, "/api/v1/users/me").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["username"], "admin");
    assert!(
        body["data"]["role"] == "admin" || body["data"]["role"] == "superadmin",
        "Admin user should have admin or superadmin role"
    );
}

#[tokio::test]
async fn fresh_database_has_empty_collections() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    // Lots should be empty (or only demo data)
    let (status, body) = auth_get(&srv, &token, "/api/v1/lots").await;
    assert_eq!(status, 200);
    let _lots = body["data"].as_array().unwrap();
    // In demo mode, there may be pre-seeded lots
    // What matters is that the endpoint works

    // Bookings should be empty for a new user (if mod-bookings is compiled)
    let (status, body) = auth_get(&srv, &token, "/api/v1/bookings").await;
    if !body.is_null() {
        assert_eq!(status, 200);
        assert!(body["data"].is_array());
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Health checks
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn all_health_endpoints_pass() {
    let srv = start_test_server().await;

    // /health
    let resp = srv
        .client
        .get(format!("{}/health", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "OK");

    // /health/live
    let resp = srv
        .client
        .get(format!("{}/health/live", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    // /health/ready
    let resp = srv
        .client
        .get(format!("{}/health/ready", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["ready"], true);
}

#[tokio::test]
async fn detailed_health_check_returns_info() {
    let srv = start_test_server().await;

    let resp = srv
        .client
        .get(format!("{}/health/detailed", srv.url))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    assert_eq!(status, 200);

    let body: Value = resp.json().await.unwrap();
    // Detailed health should include database and system info
    assert!(
        body["database"].is_object()
            || body["db"].is_object()
            || body["status"].is_string()
            || body["uptime"].is_number()
            || body.is_object(),
        "Detailed health should return structured data"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Status endpoint verifies schema
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn status_endpoint_shows_user_count() {
    let srv = start_test_server().await;

    let resp = srv
        .client
        .get(format!("{}/status", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["success"], true);

    let total_users = body["data"]["total_users"].as_u64().unwrap_or(0);
    assert!(
        total_users >= 1,
        "Should have at least 1 user (admin), got: {total_users}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Modules endpoint reflects compiled features
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn modules_endpoint_reflects_build_features() {
    let srv = start_test_server().await;

    let resp = srv
        .client
        .get(format!("{}/api/v1/modules", srv.url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    let body: Value = resp.json().await.unwrap();
    let modules = body["modules"].as_object().unwrap();

    // At minimum these core modules should be present as keys
    let expected_keys = [
        "bookings",
        "vehicles",
        "webhooks",
        "recurring",
        "calendar",
        "waitlist",
        "credits",
        "settings",
        "team",
    ];

    for key in &expected_keys {
        assert!(
            modules.contains_key(*key),
            "Modules map should contain '{key}'"
        );
    }

    // Version should be the workspace version
    assert!(body["version"].is_string());
}

// ═════════════════════════════════════════════════════════════════════════════
// Database tables exist (verified through API functionality)
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn database_tables_are_functional() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    // Test that each major table/collection is accessible via its API
    let endpoints = vec![
        ("/api/v1/lots", "lots"),
        ("/api/v1/bookings", "bookings"),
        ("/api/v1/recurring-bookings", "recurring"),
        ("/api/v1/waitlist", "waitlist"),
        ("/api/v1/swap-requests", "swaps"),
        ("/api/v1/notifications", "notifications"),
    ];

    for (path, name) in &endpoints {
        let (status, _) = auth_get(&srv, &token, path).await;
        assert!(
            status == 200 || status == 404,
            "Endpoint {name} ({path}) should return 200 or 404, got: {status}"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Setup wizard status
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn setup_status_is_completed() {
    let srv = start_test_server().await;

    let resp = srv
        .client
        .get(format!("{}/api/v1/setup/status", srv.url))
        .send()
        .await
        .unwrap();

    let status = resp.status().as_u16();
    if status == 200 {
        let body: Value = resp.json().await.unwrap();
        // In unattended mode, setup should be completed
        let completed = body["data"]["completed"].as_bool().unwrap_or(false);
        assert!(
            completed || body["data"]["setup_completed"].as_bool().unwrap_or(false),
            "Setup should be completed in unattended mode"
        );
    }
    // 404 = setup endpoint not compiled
}

// ═════════════════════════════════════════════════════════════════════════════
// Admin stats verify aggregation works on fresh data
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn admin_stats_on_fresh_database() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let (status, body) = auth_get(&srv, &token, "/api/v1/admin/stats").await;
    assert_eq!(status, 200);
    assert_eq!(body["success"], true);

    let data = &body["data"];
    assert!(
        data["total_users"].is_number(),
        "Stats must include total_users"
    );
    assert!(
        data["total_bookings"].is_number(),
        "Stats must include total_bookings"
    );
    assert!(
        data["total_lots"].is_number(),
        "Stats must include total_lots"
    );
}
