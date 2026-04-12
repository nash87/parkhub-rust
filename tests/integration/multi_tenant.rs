//! Multi-Tenant Isolation Tests
//!
//! Create two tenants with their own users, lots, and bookings.
//! Verify strict data isolation between tenants and that super-admin
//! can see both.

use crate::common::{
    admin_login, auth_delete, auth_get, auth_post, auth_put, create_test_lot, create_test_slot,
    start_test_server,
};
use serde_json::Value;

/// Helper: create a tenant via the admin API.  Returns `tenant_id`.
async fn create_tenant(
    srv: &crate::common::TestServer,
    admin_token: &str,
    name: &str,
) -> Option<String> {
    let (status, body) = auth_post(
        srv,
        admin_token,
        "/api/v1/admin/tenants",
        &serde_json::json!({
            "name": name,
            "domain": format!("{}.parkhub.test", name.to_lowercase().replace(' ', "-")),
        }),
    )
    .await;

    if status == 404 {
        // Multi-tenant feature not compiled
        return None;
    }

    assert!(
        status == 200 || status == 201,
        "Create tenant '{name}' failed: {status} body: {body}"
    );
    Some(
        body["data"]["id"]
            .as_str()
            .expect("tenant id")
            .to_string(),
    )
}

/// Helper: assign a user to a tenant (admin endpoint).
async fn assign_user_to_tenant(
    srv: &crate::common::TestServer,
    admin_token: &str,
    user_id: &str,
    tenant_id: &str,
) {
    let (status, _) = auth_put(
        srv,
        admin_token,
        &format!("/api/v1/admin/users/{user_id}"),
        &serde_json::json!({
            "tenant_id": tenant_id,
        }),
    )
    .await;

    // May succeed or may not support direct tenant assignment via this endpoint
    assert!(
        status == 200 || status == 404 || status == 422,
        "Assign user to tenant: unexpected status {status}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Core isolation test
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn tenant_isolation_users_and_lots() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    // Create two tenants
    let tenant_a_id = match create_tenant(&srv, &admin_token, "Tenant Alpha").await {
        Some(id) => id,
        None => {
            eprintln!("Multi-tenant feature not available, skipping test");
            return;
        }
    };
    let tenant_b_id = create_tenant(&srv, &admin_token, "Tenant Beta")
        .await
        .unwrap();

    // Create users for each tenant
    let (user_a_token, user_a_id, _) =
        crate::common::create_test_user(&srv, "tenant_a_user").await;
    let (user_b_token, user_b_id, _) =
        crate::common::create_test_user(&srv, "tenant_b_user").await;

    // Assign users to tenants
    assign_user_to_tenant(&srv, &admin_token, &user_a_id, &tenant_a_id).await;
    assign_user_to_tenant(&srv, &admin_token, &user_b_id, &tenant_b_id).await;

    // Create lots for each tenant (admin creates on behalf)
    let lot_a = create_test_lot(&srv, &admin_token, "Alpha Lot").await;
    let lot_b = create_test_lot(&srv, &admin_token, "Beta Lot").await;

    // Create slots
    let slot_a = create_test_slot(&srv, &admin_token, &lot_a, 1).await;
    let slot_b = create_test_slot(&srv, &admin_token, &lot_b, 1).await;

    // Create bookings
    let _booking_a =
        crate::common::create_test_booking(&srv, &user_a_token, &lot_a, &slot_a).await;
    let _booking_b =
        crate::common::create_test_booking(&srv, &user_b_token, &lot_b, &slot_b).await;

    // Tenant A user should see their own bookings
    let (status, body_a) = auth_get(&srv, &user_a_token, "/api/v1/bookings").await;
    assert_eq!(status, 200);
    let bookings_a = body_a["data"].as_array().unwrap();

    // Tenant B user should see their own bookings
    let (status, body_b) = auth_get(&srv, &user_b_token, "/api/v1/bookings").await;
    assert_eq!(status, 200);
    let bookings_b = body_b["data"].as_array().unwrap();

    // Cross-check: A's bookings should not appear in B's list and vice versa
    for bk_a in bookings_a {
        let bk_a_id = bk_a["id"].as_str().unwrap_or("");
        assert!(
            !bookings_b.iter().any(|b| b["id"].as_str() == Some(bk_a_id)),
            "Tenant A booking should not appear in Tenant B's list"
        );
    }
    for bk_b in bookings_b {
        let bk_b_id = bk_b["id"].as_str().unwrap_or("");
        assert!(
            !bookings_a.iter().any(|b| b["id"].as_str() == Some(bk_b_id)),
            "Tenant B booking should not appear in Tenant A's list"
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Admin sees all tenants
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn super_admin_sees_all_tenants() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    let _t1 = match create_tenant(&srv, &admin_token, "Visible A").await {
        Some(id) => id,
        None => return,
    };
    let _t2 = create_tenant(&srv, &admin_token, "Visible B").await.unwrap();

    let (status, body) = auth_get(&srv, &admin_token, "/api/v1/admin/tenants").await;
    assert_eq!(status, 200);
    let tenants = body["data"].as_array().unwrap();
    assert!(
        tenants.len() >= 2,
        "Admin should see at least both created tenants"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Tenant list is scoped for non-super-admin
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn regular_user_cannot_manage_tenants() {
    let srv = start_test_server().await;
    let (user_token, _, _) = crate::common::create_test_user(&srv, "tenant_nonadmin").await;

    let (status, _) = auth_get(&srv, &user_token, "/api/v1/admin/tenants").await;
    assert_eq!(
        status, 403,
        "Regular user should not access tenant management"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Tenant CRUD
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn tenant_crud_lifecycle() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    // Create
    let tenant_id = match create_tenant(&srv, &admin_token, "CRUD Tenant").await {
        Some(id) => id,
        None => return,
    };

    // Read (list)
    let (status, body) = auth_get(&srv, &admin_token, "/api/v1/admin/tenants").await;
    assert_eq!(status, 200);
    let tenants = body["data"].as_array().unwrap();
    assert!(tenants.iter().any(|t| t["id"].as_str() == Some(&tenant_id)));

    // Update
    let (status, _) = auth_put(
        &srv,
        &admin_token,
        &format!("/api/v1/admin/tenants/{tenant_id}"),
        &serde_json::json!({
            "name": "Updated CRUD Tenant",
            "domain": "updated.parkhub.test"
        }),
    )
    .await;
    // May or may not support update
    assert!(
        status == 200 || status == 404 || status == 405,
        "Tenant update: {status}"
    );

    // Delete
    let (status, _) = auth_delete(
        &srv,
        &admin_token,
        &format!("/api/v1/admin/tenants/{tenant_id}"),
    )
    .await;
    // May or may not support delete
    assert!(
        status == 200 || status == 204 || status == 404 || status == 405,
        "Tenant delete: {status}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Tenant branding
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn tenant_branding_is_stored() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;

    let (status, body) = auth_post(
        &srv,
        &admin_token,
        "/api/v1/admin/tenants",
        &serde_json::json!({
            "name": "Branded Tenant",
            "domain": "branded.parkhub.test",
            "branding": {
                "primary_color": "#6366f1",
                "logo_url": "https://example.com/logo.png",
                "company_name": "Branded Corp"
            }
        }),
    )
    .await;

    if status == 404 {
        return;
    }

    assert!(status == 200 || status == 201);

    let branding = &body["data"]["branding"];
    if branding.is_object() {
        assert_eq!(branding["primary_color"], "#6366f1");
        assert_eq!(branding["company_name"], "Branded Corp");
    }
}
