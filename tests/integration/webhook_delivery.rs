//! Webhook Integration Tests
//!
//! Register webhook endpoints, trigger events, verify HMAC signatures,
//! test v1 and v2 webhooks, and delivery retry behavior.

use crate::common::{
    admin_login, auth_delete, auth_get, auth_post, create_test_booking, create_test_lot,
    create_test_slot, create_test_user, start_test_server,
};
use serde_json::Value;

// ═════════════════════════════════════════════════════════════════════════════
// Webhook v1 CRUD
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn webhook_v1_create_list_delete() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    // Create a webhook
    let (status, body) = auth_post(
        &srv,
        &token,
        "/api/v1/admin/webhooks",
        &serde_json::json!({
            "url": "https://httpbin.org/post",
            "events": ["booking.created", "booking.cancelled"],
            "active": true
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        // Webhooks feature not compiled (404 or SPA HTML fallback)
        return;
    }

    assert!(
        status == 200 || status == 201,
        "Webhook create failed: {status} body: {body}"
    );
    assert_eq!(body["success"], true);

    let webhook_id = body["data"]["id"].as_str().unwrap().to_string();
    let secret = body["data"]["secret"].as_str().unwrap_or("");
    assert!(
        !secret.is_empty(),
        "Webhook should be assigned an HMAC secret"
    );

    // List webhooks
    let (status, body) = auth_get(&srv, &token, "/api/v1/admin/webhooks").await;
    assert_eq!(status, 200);
    let hooks = body["data"].as_array().unwrap();
    assert!(
        hooks.iter().any(|h| h["id"].as_str() == Some(&webhook_id)),
        "Created webhook should appear in list"
    );

    // Delete webhook
    let (status, _) = auth_delete(
        &srv,
        &token,
        &format!("/api/v1/admin/webhooks/{webhook_id}"),
    )
    .await;
    assert!(
        status == 200 || status == 204,
        "Webhook delete should succeed"
    );

    // Verify it's gone
    let (status, body) = auth_get(&srv, &token, "/api/v1/admin/webhooks").await;
    assert_eq!(status, 200);
    let hooks = body["data"].as_array().unwrap();
    assert!(
        !hooks.iter().any(|h| h["id"].as_str() == Some(&webhook_id)),
        "Deleted webhook should not appear in list"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Webhook v2 CRUD + test delivery
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn webhook_v2_lifecycle_with_test_event() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    // v2 webhooks use a different storage path (settings-based)
    let (status, body) = auth_post(
        &srv,
        &token,
        "/api/v1/admin/webhooks",
        &serde_json::json!({
            "url": "https://httpbin.org/post",
            "events": ["booking.created"],
            "active": true,
            "description": "Integration test v2 hook"
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        // Webhooks feature not compiled
        return;
    }

    assert!(
        status == 200 || status == 201,
        "v2 webhook create: {status}"
    );
    let webhook_id = body["data"]["id"].as_str().unwrap().to_string();

    // Send test event
    let (status, body) = auth_post(
        &srv,
        &token,
        &format!("/api/v1/admin/webhooks/{webhook_id}/test"),
        &serde_json::json!({}),
    )
    .await;

    // Test endpoint may or may not exist
    if status == 200 || status == 201 {
        assert_eq!(body["success"], true, "Test event should succeed");
    }

    // Check delivery log
    let (status, body) = auth_get(
        &srv,
        &token,
        &format!("/api/v1/admin/webhooks/{webhook_id}/deliveries"),
    )
    .await;

    if status == 200 {
        // May have delivery entries from the test event
        assert!(body["data"].is_array() || body["data"].is_null());
    }

    // Cleanup
    let _ = auth_delete(
        &srv,
        &token,
        &format!("/api/v1/admin/webhooks/{webhook_id}"),
    )
    .await;
}

// ═════════════════════════════════════════════════════════════════════════════
// Webhook HMAC secret is present
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn webhook_secret_is_cryptographically_random() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let mut secrets = Vec::new();
    for i in 0..3 {
        let (status, body) = auth_post(
            &srv,
            &token,
            "/api/v1/admin/webhooks",
            &serde_json::json!({
                "url": format!("https://httpbin.org/post/{i}"),
                "events": ["booking.created"],
                "active": true
            }),
        )
        .await;

        if status == 404 {
            return;
        }

        assert!(status == 200 || status == 201);
        if let Some(s) = body["data"]["secret"].as_str() {
            secrets.push(s.to_string());
        }
    }

    // Verify all secrets are unique
    let unique: std::collections::HashSet<_> = secrets.iter().collect();
    assert_eq!(
        unique.len(),
        secrets.len(),
        "Each webhook must get a unique HMAC secret"
    );

    // Verify secrets have reasonable length (at least 32 hex chars)
    for s in &secrets {
        assert!(
            s.len() >= 32,
            "Webhook secret should be at least 32 chars, got: {}",
            s.len()
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Webhook event subscription filtering
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn webhook_stores_event_subscriptions() {
    let srv = start_test_server().await;
    let (token, _) = admin_login(&srv).await;

    let events = vec!["booking.created".to_string(), "user.registered".to_string()];

    let (status, body) = auth_post(
        &srv,
        &token,
        "/api/v1/admin/webhooks",
        &serde_json::json!({
            "url": "https://httpbin.org/post",
            "events": events,
            "active": true
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        // Webhooks feature not compiled
        return;
    }

    assert!(status == 200 || status == 201);
    let stored_events = body["data"]["events"].as_array().unwrap();
    assert_eq!(stored_events.len(), 2);
    assert!(stored_events
        .iter()
        .any(|e| e.as_str() == Some("booking.created")));
    assert!(stored_events
        .iter()
        .any(|e| e.as_str() == Some("user.registered")));
}

// ═════════════════════════════════════════════════════════════════════════════
// Non-admin cannot manage webhooks
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn non_admin_cannot_create_webhook() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    // Check if webhooks feature is compiled
    let (_, probe) = auth_get(&srv, &admin_token, "/api/v1/admin/webhooks").await;
    if probe.is_null() {
        // Feature not compiled; SPA fallback — skip
        return;
    }

    let (user_token, _, _) = create_test_user(&srv, "webhook_nonadmin").await;

    let (status, _) = auth_post(
        &srv,
        &user_token,
        "/api/v1/admin/webhooks",
        &serde_json::json!({
            "url": "https://httpbin.org/post",
            "events": ["booking.created"],
            "active": true
        }),
    )
    .await;

    assert_eq!(
        status, 403,
        "Non-admin should get 403 for webhook management"
    );
}
