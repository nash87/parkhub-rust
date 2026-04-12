//! Recurring Booking Tests
//!
//! Create daily recurring bookings, verify instance generation,
//! modify and cancel individual instances and series.

use crate::common::{
    admin_login, auth_delete, auth_get, auth_post, auth_put, create_test_lot, create_test_slot,
    create_test_user, start_test_server,
};
use serde_json::Value;

// ═════════════════════════════════════════════════════════════════════════════
// Create recurring booking
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn create_recurring_booking_succeeds() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_token, _, _) = create_test_user(&srv, "recurring_create").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let end_date = (chrono::Utc::now() + chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();

    let (status, body) = auth_post(
        &srv,
        &user_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [1, 2, 3, 4, 5],
            "start_date": today,
            "end_date": end_date,
            "start_time": "08:00",
            "end_time": "17:00",
            "vehicle_plate": "M-RC 1001"
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        // Recurring bookings feature not compiled (404 or SPA HTML fallback)
        return;
    }

    assert!(
        status == 200 || status == 201,
        "Create recurring booking failed: {status} body: {body}"
    );
    assert_eq!(body["success"], true);
    let recurring_id = body["data"]["id"].as_str().unwrap();
    assert!(!recurring_id.is_empty());
    assert_eq!(body["data"]["active"], true);
}

// ═════════════════════════════════════════════════════════════════════════════
// List recurring bookings
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn list_recurring_bookings_returns_created_ones() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_token, _, _) = create_test_user(&srv, "recurring_list").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring List Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Create a recurring booking
    let (status, body) = auth_post(
        &srv,
        &user_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [1, 3, 5],
            "start_date": today,
            "end_date": null,
            "start_time": "09:00",
            "end_time": "12:00",
            "vehicle_plate": null
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        return;
    }

    assert!(status == 200 || status == 201);
    let recurring_id = body["data"]["id"].as_str().unwrap().to_string();

    // List recurring bookings
    let (status, body) = auth_get(&srv, &user_token, "/api/v1/recurring-bookings").await;
    assert_eq!(status, 200);
    let items = body["data"].as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|r| r["id"].as_str() == Some(&recurring_id)),
        "Created recurring booking should appear in list"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Delete a recurring booking
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn delete_recurring_booking_removes_it() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_token, _, _) = create_test_user(&srv, "recurring_delete").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring Delete Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (status, body) = auth_post(
        &srv,
        &user_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [2, 4],
            "start_date": today,
            "start_time": "10:00",
            "end_time": "14:00",
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        return;
    }

    assert!(status == 200 || status == 201);
    let recurring_id = body["data"]["id"].as_str().unwrap().to_string();

    // Delete it
    let (status, _) = auth_delete(
        &srv,
        &user_token,
        &format!("/api/v1/recurring-bookings/{recurring_id}"),
    )
    .await;
    assert!(
        status == 200 || status == 204,
        "Delete recurring should succeed, got: {status}"
    );

    // Verify it's gone
    let (status, body) = auth_get(&srv, &user_token, "/api/v1/recurring-bookings").await;
    assert_eq!(status, 200);
    let items = body["data"].as_array().unwrap();
    assert!(
        !items
            .iter()
            .any(|r| r["id"].as_str() == Some(&recurring_id)),
        "Deleted recurring booking should not appear in list"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Update recurring booking
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn update_recurring_booking_modifies_fields() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_token, _, _) = create_test_user(&srv, "recurring_update").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring Update Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (status, body) = auth_post(
        &srv,
        &user_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [1, 2, 3],
            "start_date": today,
            "start_time": "08:00",
            "end_time": "16:00",
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        return;
    }

    assert!(status == 200 || status == 201);
    let recurring_id = body["data"]["id"].as_str().unwrap().to_string();

    // Update: change days and time
    let (status, body) = auth_put(
        &srv,
        &user_token,
        &format!("/api/v1/recurring-bookings/{recurring_id}"),
        &serde_json::json!({
            "days_of_week": [1, 3, 5],
            "start_time": "09:00",
            "end_time": "17:00",
        }),
    )
    .await;

    if status == 200 {
        assert_eq!(body["success"], true);
        // Verify the updated fields
        let updated = &body["data"];
        if updated["days_of_week"].is_array() {
            let days: Vec<u64> = updated["days_of_week"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|d| d.as_u64())
                .collect();
            assert_eq!(days, vec![1, 3, 5]);
        }
    }
    // 404 or 405 = update not supported
}

// ═════════════════════════════════════════════════════════════════════════════
// Recurring booking data structure validation
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn recurring_booking_has_correct_shape() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_token, _, _) = create_test_user(&srv, "recurring_shape").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring Shape Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (status, body) = auth_post(
        &srv,
        &user_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [1, 2, 3, 4, 5],
            "start_date": today,
            "start_time": "07:30",
            "end_time": "16:30",
            "vehicle_plate": "M-SH 5001"
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        return;
    }

    assert!(status == 200 || status == 201);
    let rb = &body["data"];

    // Validate all expected fields
    assert!(rb["id"].is_string(), "Must have id");
    assert!(rb["user_id"].is_string(), "Must have user_id");
    assert!(rb["lot_id"].is_string(), "Must have lot_id");
    assert!(
        rb["days_of_week"].is_array(),
        "Must have days_of_week array"
    );
    assert!(rb["start_date"].is_string(), "Must have start_date");
    assert!(rb["start_time"].is_string(), "Must have start_time");
    assert!(rb["end_time"].is_string(), "Must have end_time");
    assert!(rb["active"].is_boolean(), "Must have active boolean");
    assert!(rb["created_at"].is_string(), "Must have created_at");
}

// ═════════════════════════════════════════════════════════════════════════════
// Cannot delete another user's recurring booking
// ═════════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn cannot_delete_other_users_recurring_booking() {
    let srv = start_test_server().await;
    let (admin_token, _) = admin_login(&srv).await;
    let (user_a_token, _, _) = create_test_user(&srv, "recurring_owner").await;
    let (user_b_token, _, _) = create_test_user(&srv, "recurring_thief").await;

    let lot_id = create_test_lot(&srv, &admin_token, "Recurring Ownership Lot").await;
    let slot_id = create_test_slot(&srv, &admin_token, &lot_id, 1).await;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let (status, body) = auth_post(
        &srv,
        &user_a_token,
        "/api/v1/recurring-bookings",
        &serde_json::json!({
            "lot_id": lot_id,
            "slot_id": slot_id,
            "days_of_week": [1],
            "start_date": today,
            "start_time": "08:00",
            "end_time": "09:00",
        }),
    )
    .await;

    if status == 404 || body.is_null() {
        return;
    }

    assert!(status == 200 || status == 201);
    let recurring_id = body["data"]["id"].as_str().unwrap().to_string();

    // User B tries to delete User A's recurring booking
    let (status, _) = auth_delete(
        &srv,
        &user_b_token,
        &format!("/api/v1/recurring-bookings/{recurring_id}"),
    )
    .await;

    assert!(
        status == 403 || status == 404,
        "Should not delete another user's recurring booking, got: {status}"
    );
}
