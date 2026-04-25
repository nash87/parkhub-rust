//! Unit tests for the `Database` across all domain sub-modules.
//!
//! Kept as a single `mod tests` file per the T-1740 split spec: relocating the
//! original `#[cfg(test)] mod tests` content verbatim preserves the 1729 test
//! count and avoids helper-name collisions between domain test suites.

use super::*;
use parkhub_common::models::{SlotFeature, SlotPosition, SlotStatus, SlotType};
use std::path::PathBuf;
use tempfile::tempdir;

use parkhub_common::models::{
    Absence, Announcement, Booking, Notification, ParkingLot, ParkingSlot, User, Vehicle,
};

fn test_config(path: PathBuf, encrypted: bool) -> DatabaseConfig {
    DatabaseConfig {
        path,
        encryption_enabled: encrypted,
        passphrase: if encrypted {
            Some("test-passphrase".to_string())
        } else {
            None
        },
        create_if_missing: true,
    }
}

#[tokio::test]
async fn test_database_create() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();
    assert!(!db.is_encrypted());
    assert!(db.is_fresh().await.unwrap());
}

#[tokio::test]
async fn test_database_encrypted() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), true);
    let db = Database::open(&config).unwrap();
    assert!(db.is_encrypted());
}

#[tokio::test]
async fn test_setup_completed() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    assert!(db.is_fresh().await.unwrap());
    db.mark_setup_completed().await.unwrap();
    assert!(!db.is_fresh().await.unwrap());
}

#[tokio::test]
async fn test_settings() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    assert!(db.get_setting("test_key").await.unwrap().is_none());
    db.set_setting("test_key", "test_value").await.unwrap();
    assert_eq!(
        db.get_setting("test_key").await.unwrap(),
        Some("test_value".to_string())
    );
}

#[tokio::test]
async fn test_stats() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    let stats = db.stats().await.unwrap();
    assert_eq!(stats.users, 0);
    assert_eq!(stats.bookings, 0);
    assert_eq!(stats.parking_lots, 0);
}

#[tokio::test]
async fn test_push_subscriptions_crud() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    let user_id = Uuid::new_v4();
    let other_user = Uuid::new_v4();

    // No subscriptions initially
    let subs = db.get_push_subscriptions_by_user(&user_id).await.unwrap();
    assert!(subs.is_empty());

    // Save two subscriptions for user_id
    let sub1 = PushSubscription {
        id: Uuid::new_v4(),
        user_id,
        endpoint: "https://push.example.com/sub1".into(),
        p256dh: "key1".into(),
        auth: "auth1".into(),
        created_at: Utc::now(),
    };
    let sub2 = PushSubscription {
        id: Uuid::new_v4(),
        user_id,
        endpoint: "https://push.example.com/sub2".into(),
        p256dh: "key2".into(),
        auth: "auth2".into(),
        created_at: Utc::now(),
    };
    // Save one subscription for other_user
    let sub3 = PushSubscription {
        id: Uuid::new_v4(),
        user_id: other_user,
        endpoint: "https://push.example.com/sub3".into(),
        p256dh: "key3".into(),
        auth: "auth3".into(),
        created_at: Utc::now(),
    };

    db.save_push_subscription(&sub1).await.unwrap();
    db.save_push_subscription(&sub2).await.unwrap();
    db.save_push_subscription(&sub3).await.unwrap();

    // List for user_id -> 2
    let subs = db.get_push_subscriptions_by_user(&user_id).await.unwrap();
    assert_eq!(subs.len(), 2);

    // List all -> 3
    let all = db.list_all_push_subscriptions().await.unwrap();
    assert_eq!(all.len(), 3);

    // Delete user_id subscriptions
    let deleted = db
        .delete_push_subscriptions_by_user(&user_id)
        .await
        .unwrap();
    assert_eq!(deleted, 2);

    // user_id has none, other_user still has 1
    assert!(
        db.get_push_subscriptions_by_user(&user_id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.get_push_subscriptions_by_user(&other_user)
            .await
            .unwrap()
            .len(),
        1
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// WEBHOOK CRUD
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_webhook_crud() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    // Empty initially
    let all = db.list_webhooks().await.unwrap();
    assert!(all.is_empty());

    // Create two webhooks
    let wh1 = Webhook {
        id: Uuid::new_v4(),
        url: "https://example.com/hooks/parking".to_string(),
        secret: "sec_abc123".to_string(),
        events: vec!["booking.created".into(), "booking.cancelled".into()],
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let wh2 = Webhook {
        id: Uuid::new_v4(),
        url: "https://other.io/webhooks".to_string(),
        secret: "sec_xyz789".to_string(),
        events: vec!["slot.status_changed".into()],
        active: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    db.save_webhook(&wh1).await.unwrap();
    db.save_webhook(&wh2).await.unwrap();

    // List returns both
    let all = db.list_webhooks().await.unwrap();
    assert_eq!(all.len(), 2);

    // Get by ID
    let fetched = db.get_webhook(&wh1.id.to_string()).await.unwrap().unwrap();
    assert_eq!(fetched.url, "https://example.com/hooks/parking");
    assert_eq!(fetched.events.len(), 2);
    assert!(fetched.active);

    // Get non-existent returns None
    let missing = db.get_webhook(&Uuid::new_v4().to_string()).await.unwrap();
    assert!(missing.is_none());

    // Delete first webhook
    let deleted = db.delete_webhook(&wh1.id.to_string()).await.unwrap();
    assert!(deleted);

    // Second delete of same ID returns false
    let deleted_again = db.delete_webhook(&wh1.id.to_string()).await.unwrap();
    assert!(!deleted_again);

    // Only wh2 remains
    let remaining = db.list_webhooks().await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, wh2.id);
}

// ═══════════════════════════════════════════════════════════════════════════
// ZONE CRUD
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_zone_crud() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    let lot_a = Uuid::new_v4();
    let lot_b = Uuid::new_v4();

    // No zones initially
    let zones = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
    assert!(zones.is_empty());

    // Create zones in lot_a
    let z1 = Zone {
        id: Uuid::new_v4(),
        lot_id: lot_a,
        name: "Level A".to_string(),
        description: Some("Ground floor, near entrance".to_string()),
        color: Some("#4CAF50".to_string()),
        created_at: Utc::now(),
    };
    let z2 = Zone {
        id: Uuid::new_v4(),
        lot_id: lot_a,
        name: "VIP Section".to_string(),
        description: None,
        color: Some("#FFD700".to_string()),
        created_at: Utc::now(),
    };
    // Zone in a different lot
    let z3 = Zone {
        id: Uuid::new_v4(),
        lot_id: lot_b,
        name: "Basement B1".to_string(),
        description: Some("Underground level".to_string()),
        color: None,
        created_at: Utc::now(),
    };

    db.save_zone(&z1).await.unwrap();
    db.save_zone(&z2).await.unwrap();
    db.save_zone(&z3).await.unwrap();

    // List by lot_a -> 2
    let zones_a = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
    assert_eq!(zones_a.len(), 2);
    let names: Vec<&str> = zones_a.iter().map(|z| z.name.as_str()).collect();
    assert!(names.contains(&"Level A"));
    assert!(names.contains(&"VIP Section"));

    // List by lot_b -> 1
    let zones_b = db.list_zones_by_lot(&lot_b.to_string()).await.unwrap();
    assert_eq!(zones_b.len(), 1);
    assert_eq!(zones_b[0].name, "Basement B1");

    // Delete z1 from lot_a
    let deleted = db
        .delete_zone(&lot_a.to_string(), &z1.id.to_string())
        .await
        .unwrap();
    assert!(deleted);

    // Delete non-existent zone returns false
    let no_delete = db
        .delete_zone(&lot_a.to_string(), &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!no_delete);

    // Only z2 remains in lot_a
    let zones_a = db.list_zones_by_lot(&lot_a.to_string()).await.unwrap();
    assert_eq!(zones_a.len(), 1);
    assert_eq!(zones_a[0].name, "VIP Section");

    // lot_b untouched
    assert_eq!(
        db.list_zones_by_lot(&lot_b.to_string())
            .await
            .unwrap()
            .len(),
        1
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AUDIT LOG
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_audit_log() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    // Empty initially
    let entries = db.list_audit_log(10).await.unwrap();
    assert!(entries.is_empty());

    let user_id = Uuid::new_v4();

    // Insert entries with staggered timestamps so ordering is deterministic
    let base = Utc::now();
    let e1 = AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: base - chrono::Duration::seconds(30),
        event_type: "user.login".to_string(),
        user_id: Some(user_id),
        username: Some("alice".to_string()),
        details: Some("Login from 192.168.1.10".to_string()),
        target_type: None,
        target_id: None,
        ip_address: Some("192.168.1.10".to_string()),
    };
    let e2 = AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: base - chrono::Duration::seconds(20),
        event_type: "booking.created".to_string(),
        user_id: Some(user_id),
        username: Some("alice".to_string()),
        details: Some("Booked slot A-12 for 2h".to_string()),
        target_type: Some("booking".to_string()),
        target_id: Some("A-12".to_string()),
        ip_address: None,
    };
    let e3 = AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: base - chrono::Duration::seconds(10),
        event_type: "admin.reset".to_string(),
        user_id: None,
        username: None,
        details: Some("Demo data reset triggered".to_string()),
        target_type: None,
        target_id: None,
        ip_address: None,
    };
    let e4 = AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: base,
        event_type: "user.logout".to_string(),
        user_id: Some(user_id),
        username: Some("alice".to_string()),
        details: None,
        target_type: None,
        target_id: None,
        ip_address: None,
    };

    db.save_audit_log(&e1).await.unwrap();
    db.save_audit_log(&e2).await.unwrap();
    db.save_audit_log(&e3).await.unwrap();
    db.save_audit_log(&e4).await.unwrap();

    // List all 4
    let all = db.list_audit_log(100).await.unwrap();
    assert_eq!(all.len(), 4);

    // Most recent first
    assert_eq!(all[0].event_type, "user.logout");
    assert_eq!(all[1].event_type, "admin.reset");
    assert_eq!(all[2].event_type, "booking.created");
    assert_eq!(all[3].event_type, "user.login");

    // Limit truncates
    let limited = db.list_audit_log(2).await.unwrap();
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].event_type, "user.logout");
    assert_eq!(limited[1].event_type, "admin.reset");

    // Verify entry fields
    let login = &all[3];
    assert_eq!(login.user_id, Some(user_id));
    assert_eq!(login.username.as_deref(), Some("alice"));
    assert_eq!(login.details.as_deref(), Some("Login from 192.168.1.10"));
}

// ═══════════════════════════════════════════════════════════════════════════
// CLEAR ALL DATA — new tables included
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_clear_all_data_includes_new_tables() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    // Populate webhooks
    let wh = Webhook {
        id: Uuid::new_v4(),
        url: "https://hooks.test/a".to_string(),
        secret: "s".to_string(),
        events: vec!["booking.created".into()],
        active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    db.save_webhook(&wh).await.unwrap();

    // Populate zones
    let lot_id = Uuid::new_v4();
    let zone = Zone {
        id: Uuid::new_v4(),
        lot_id,
        name: "Zone-1".to_string(),
        description: None,
        color: None,
        created_at: Utc::now(),
    };
    db.save_zone(&zone).await.unwrap();

    // Populate favorites
    let fav = Favorite {
        user_id: Uuid::new_v4(),
        slot_id: Uuid::new_v4(),
        lot_id,
        created_at: Utc::now(),
    };
    db.save_favorite(&fav).await.unwrap();

    // Populate audit log
    let entry = AuditLogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        event_type: "test.event".to_string(),
        user_id: None,
        username: None,
        details: None,
        target_type: None,
        target_id: None,
        ip_address: None,
    };
    db.save_audit_log(&entry).await.unwrap();

    // Verify data exists
    assert_eq!(db.list_webhooks().await.unwrap().len(), 1);
    assert_eq!(
        db.list_zones_by_lot(&lot_id.to_string())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db.list_favorites_by_user(&fav.user_id.to_string())
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(db.list_audit_log(10).await.unwrap().len(), 1);

    // Clear everything
    db.clear_all_data().await.unwrap();

    // All new tables must be empty
    assert!(db.list_webhooks().await.unwrap().is_empty());
    assert!(
        db.list_zones_by_lot(&lot_id.to_string())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        db.list_favorites_by_user(&fav.user_id.to_string())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(db.list_audit_log(10).await.unwrap().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// DELETE PARKING SLOT
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_delete_parking_slot() {
    let dir = tempdir().unwrap();
    let config = test_config(dir.path().to_path_buf(), false);
    let db = Database::open(&config).unwrap();

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();

    let slot1 = ParkingSlot {
        id: Uuid::new_v4(),
        lot_id,
        floor_id,
        slot_number: 1,
        row: 0,
        column: 0,
        slot_type: SlotType::Standard,
        status: SlotStatus::Available,
        current_booking: None,
        features: vec![SlotFeature::NearExit, SlotFeature::WellLit],
        position: SlotPosition {
            x: 10.0,
            y: 20.0,
            width: 3.0,
            height: 5.0,
            rotation: 0.0,
        },
        is_accessible: false,
    };
    let slot2 = ParkingSlot {
        id: Uuid::new_v4(),
        lot_id,
        floor_id,
        slot_number: 2,
        row: 0,
        column: 1,
        slot_type: SlotType::Electric,
        status: SlotStatus::Available,
        current_booking: None,
        features: vec![SlotFeature::ChargingStation],
        position: SlotPosition {
            x: 14.0,
            y: 20.0,
            width: 3.0,
            height: 5.0,
            rotation: 0.0,
        },
        is_accessible: false,
    };

    db.save_parking_slot(&slot1).await.unwrap();
    db.save_parking_slot(&slot2).await.unwrap();

    // Both slots exist
    assert!(
        db.get_parking_slot(&slot1.id.to_string())
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        db.get_parking_slot(&slot2.id.to_string())
            .await
            .unwrap()
            .is_some()
    );
    let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
    assert_eq!(by_lot.len(), 2);

    // Delete slot1
    let removed = db.delete_parking_slot(&slot1.id.to_string()).await.unwrap();
    assert!(removed);

    // slot1 gone from primary table
    assert!(
        db.get_parking_slot(&slot1.id.to_string())
            .await
            .unwrap()
            .is_none()
    );

    // slot1 gone from index (lot query returns only slot2)
    let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
    assert_eq!(by_lot.len(), 1);
    assert_eq!(by_lot[0].id, slot2.id);

    // slot2 unaffected
    let s2 = db
        .get_parking_slot(&slot2.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(s2.slot_number, 2);

    // Deleting non-existent slot returns false
    let no_op = db
        .delete_parking_slot(&Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!no_op);

    // Double-delete returns false
    let again = db.delete_parking_slot(&slot1.id.to_string()).await.unwrap();
    assert!(!again);
}

// ═══════════════════════════════════════════════════════════════════════════
// USER CRUD
// ═══════════════════════════════════════════════════════════════════════════

fn make_user(username: &str, email: &str) -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        username: username.to_string(),
        email: email.to_string(),
        password_hash: "$argon2id$v=19$m=65536,t=3,p=4$fake".to_string(),
        name: format!("{} User", username),
        picture: None,
        phone: None,
        role: parkhub_common::models::UserRole::User,
        created_at: now,
        updated_at: now,
        last_login: None,
        preferences: parkhub_common::models::UserPreferences::default(),
        is_active: true,
        credits_balance: 0,
        credits_monthly_quota: 40,
        credits_last_refilled: None,
        tenant_id: None,
        accessibility_needs: None,
        cost_center: None,
        department: None,
        settings: None,
    }
}

fn make_vehicle(user_id: Uuid, plate: &str) -> Vehicle {
    Vehicle {
        id: Uuid::new_v4(),
        user_id,
        license_plate: plate.to_string(),
        make: Some("Tesla".to_string()),
        model: Some("Model 3".to_string()),
        color: Some("White".to_string()),
        vehicle_type: parkhub_common::models::VehicleType::Electric,
        fuel_type: parkhub_common::FuelType::Unknown,
        is_default: true,
        created_at: Utc::now(),
    }
}

fn make_booking(user_id: Uuid, lot_id: Uuid, vehicle: &Vehicle) -> Booking {
    let now = Utc::now();
    Booking {
        id: Uuid::new_v4(),
        user_id,
        lot_id,
        slot_id: Uuid::new_v4(),
        slot_number: 1,
        floor_name: "Ground".to_string(),
        vehicle: vehicle.clone(),
        start_time: now,
        end_time: now + chrono::Duration::hours(2),
        status: parkhub_common::models::BookingStatus::Confirmed,
        pricing: parkhub_common::models::BookingPricing {
            base_price: 5.0,
            discount: 0.0,
            tax: 0.95,
            total: 5.95,
            currency: "EUR".to_string(),
            payment_status: parkhub_common::models::PaymentStatus::Paid,
            payment_method: Some("card".to_string()),
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: None,
        notes: None,
        tenant_id: None,
    }
}

fn make_slot(lot_id: Uuid, floor_id: Uuid, number: i32) -> ParkingSlot {
    ParkingSlot {
        id: Uuid::new_v4(),
        lot_id,
        floor_id,
        slot_number: number,
        row: 0,
        column: number,
        slot_type: SlotType::Standard,
        status: SlotStatus::Available,
        current_booking: None,
        features: vec![],
        position: SlotPosition {
            x: number as f32 * 4.0,
            y: 0.0,
            width: 3.0,
            height: 5.0,
            rotation: 0.0,
        },
        is_accessible: false,
    }
}

fn make_parking_lot() -> ParkingLot {
    let now = Utc::now();
    ParkingLot {
        id: Uuid::new_v4(),
        name: "Test Lot".to_string(),
        address: "123 Test St".to_string(),
        latitude: 48.1351,
        longitude: 11.582,
        total_slots: 50,
        available_slots: 50,
        floors: vec![],
        amenities: vec!["EV Charging".to_string()],
        pricing: parkhub_common::models::PricingInfo {
            currency: "EUR".to_string(),
            rates: vec![],
            daily_max: Some(20.0),
            monthly_pass: Some(150.0),
        },
        operating_hours: parkhub_common::models::OperatingHours {
            is_24h: true,
            monday: None,
            tuesday: None,
            wednesday: None,
            thursday: None,
            friday: None,
            saturday: None,
            sunday: None,
        },
        images: vec![],
        status: parkhub_common::models::LotStatus::Open,
        created_at: now,
        updated_at: now,
        tenant_id: None,
    }
}

#[tokio::test]
async fn test_user_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let mut user = make_user("alice", "alice@example.com");

    // Create
    db.save_user(&user).await.unwrap();

    // Get by ID
    let fetched = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
    assert_eq!(fetched.username, "alice");
    assert_eq!(fetched.email, "alice@example.com");

    // Get by username
    let by_name = db.get_user_by_username("alice").await.unwrap().unwrap();
    assert_eq!(by_name.id, user.id);

    // Get by email
    let by_email = db
        .get_user_by_email("alice@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_email.id, user.id);

    // Update
    user.name = "Alice Updated".to_string();
    user.updated_at = Utc::now();
    db.save_user(&user).await.unwrap();
    let updated = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
    assert_eq!(updated.name, "Alice Updated");

    // Delete
    let deleted = db.delete_user(&user.id.to_string()).await.unwrap();
    assert!(deleted);
    assert!(db.get_user(&user.id.to_string()).await.unwrap().is_none());
    assert!(db.get_user_by_username("alice").await.unwrap().is_none());
    assert!(
        db.get_user_by_email("alice@example.com")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_user_list() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let u1 = make_user("alice", "alice@test.com");
    let u2 = make_user("bob", "bob@test.com");
    let u3 = make_user("charlie", "charlie@test.com");

    db.save_user(&u1).await.unwrap();
    db.save_user(&u2).await.unwrap();
    db.save_user(&u3).await.unwrap();

    let all = db.list_users().await.unwrap();
    assert_eq!(all.len(), 3);
    let names: Vec<&str> = all.iter().map(|u| u.username.as_str()).collect();
    assert!(names.contains(&"alice"));
    assert!(names.contains(&"bob"));
    assert!(names.contains(&"charlie"));
}

#[tokio::test]
async fn test_user_not_found() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let fake_id = Uuid::new_v4().to_string();
    assert!(db.get_user(&fake_id).await.unwrap().is_none());
    assert!(
        db.get_user_by_username("nonexistent")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        db.get_user_by_email("nobody@nowhere.com")
            .await
            .unwrap()
            .is_none()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// BOOKING OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_booking_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user = make_user("parker", "parker@test.com");
    let vehicle = make_vehicle(user.id, "M-PH 1234");
    let lot_id = Uuid::new_v4();
    let booking = make_booking(user.id, lot_id, &vehicle);

    // Create
    db.save_booking(&booking).await.unwrap();

    // Get
    let fetched = db
        .get_booking(&booking.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.user_id, user.id);
    assert_eq!(fetched.lot_id, lot_id);
    assert_eq!(fetched.vehicle.license_plate, "M-PH 1234");

    // List by user
    let by_user = db
        .list_bookings_by_user(&user.id.to_string())
        .await
        .unwrap();
    assert_eq!(by_user.len(), 1);
    assert_eq!(by_user[0].id, booking.id);

    // List all
    let all = db.list_bookings().await.unwrap();
    assert_eq!(all.len(), 1);

    // Delete
    let deleted = db.delete_booking(&booking.id.to_string()).await.unwrap();
    assert!(deleted);
    assert!(
        db.get_booking(&booking.id.to_string())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_booking_by_lot() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user = make_user("parker", "parker@test.com");
    let vehicle = make_vehicle(user.id, "M-PH 5678");
    let lot_a = Uuid::new_v4();
    let lot_b = Uuid::new_v4();

    let b1 = make_booking(user.id, lot_a, &vehicle);
    let b2 = make_booking(user.id, lot_a, &vehicle);
    let b3 = make_booking(user.id, lot_b, &vehicle);

    db.save_booking(&b1).await.unwrap();
    db.save_booking(&b2).await.unwrap();
    db.save_booking(&b3).await.unwrap();

    let all = db.list_bookings().await.unwrap();
    assert_eq!(all.len(), 3);

    let lot_a_bookings: Vec<_> = all.iter().filter(|b| b.lot_id == lot_a).collect();
    let lot_b_bookings: Vec<_> = all.iter().filter(|b| b.lot_id == lot_b).collect();
    assert_eq!(lot_a_bookings.len(), 2);
    assert_eq!(lot_b_bookings.len(), 1);
    assert_eq!(lot_b_bookings[0].id, b3.id);
}

// ═══════════════════════════════════════════════════════════════════════════
// VEHICLE OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_vehicle_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let vehicle = make_vehicle(user_id, "B-AB 9876");

    // Create
    db.save_vehicle(&vehicle).await.unwrap();

    // Get
    let fetched = db
        .get_vehicle(&vehicle.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.license_plate, "B-AB 9876");
    assert_eq!(fetched.user_id, user_id);

    // List by user
    let by_user = db
        .list_vehicles_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert_eq!(by_user.len(), 1);

    // Delete
    let deleted = db.delete_vehicle(&vehicle.id.to_string()).await.unwrap();
    assert!(deleted);
    assert!(
        db.get_vehicle(&vehicle.id.to_string())
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_vehicle_delete_nonexistent() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let result = db
        .delete_vehicle(&Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!result);
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_session_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let session = Session::new(user_id, 24, "testuser", "user");
    let token = "access_tok_abc123";

    // Save
    db.save_session(token, &session).await.unwrap();

    // Get
    let fetched = db.get_session(token).await.unwrap().unwrap();
    assert_eq!(fetched.user_id, user_id);
    assert_eq!(fetched.username, "testuser");
    assert_eq!(fetched.role, "user");
    assert!(!fetched.is_expired());

    // Delete
    let deleted = db.delete_session(token).await.unwrap();
    assert!(deleted);
    assert!(db.get_session(token).await.unwrap().is_none());

    // Delete again returns false
    let again = db.delete_session(token).await.unwrap();
    assert!(!again);
}

#[tokio::test]
async fn test_session_expiry() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    // Create a session that expired 1 hour ago
    let mut session = Session::new(user_id, 1, "expired_user", "user");
    session.expires_at = Utc::now() - chrono::Duration::hours(1);
    let token = "expired_token_xyz";

    db.save_session(token, &session).await.unwrap();

    // is_expired should be true on the raw struct
    assert!(session.is_expired());

    // get_session filters out expired sessions -> returns None
    assert!(db.get_session(token).await.unwrap().is_none());
}

#[tokio::test]
async fn test_delete_sessions_by_user() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    let s1 = Session::new(user_a, 24, "alice", "user");
    let s2 = Session::new(user_a, 24, "alice", "user");
    let s3 = Session::new(user_a, 24, "alice", "user");
    let s4 = Session::new(user_b, 24, "bob", "admin");

    db.save_session("tok_a1", &s1).await.unwrap();
    db.save_session("tok_a2", &s2).await.unwrap();
    db.save_session("tok_a3", &s3).await.unwrap();
    db.save_session("tok_b1", &s4).await.unwrap();

    // Delete all sessions for user_a
    let deleted = db.delete_sessions_by_user(user_a).await.unwrap();
    assert_eq!(deleted, 3);

    // user_a sessions gone
    assert!(db.get_session("tok_a1").await.unwrap().is_none());
    assert!(db.get_session("tok_a2").await.unwrap().is_none());
    assert!(db.get_session("tok_a3").await.unwrap().is_none());

    // user_b session untouched
    let bob = db.get_session("tok_b1").await.unwrap().unwrap();
    assert_eq!(bob.user_id, user_b);
}

// ═══════════════════════════════════════════════════════════════════════════
// SETTINGS OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_settings_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    // Non-existent key returns None
    assert!(db.get_setting("theme").await.unwrap().is_none());

    // Set
    db.set_setting("theme", "dark").await.unwrap();
    assert_eq!(
        db.get_setting("theme").await.unwrap(),
        Some("dark".to_string())
    );

    // Overwrite
    db.set_setting("theme", "light").await.unwrap();
    assert_eq!(
        db.get_setting("theme").await.unwrap(),
        Some("light".to_string())
    );

    // Another key is independent
    assert!(db.get_setting("locale").await.unwrap().is_none());
}

#[tokio::test]
async fn test_setup_workflow() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    // Fresh DB
    assert!(db.is_fresh().await.unwrap());

    // Mark setup completed
    db.mark_setup_completed().await.unwrap();
    assert!(!db.is_fresh().await.unwrap());

    // Idempotent — marking again doesn't fail
    db.mark_setup_completed().await.unwrap();
    assert!(!db.is_fresh().await.unwrap());
}

// ═══════════════════════════════════════════════════════════════════════════
// NOTIFICATION OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_notification_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let notif = Notification {
        id: Uuid::new_v4(),
        user_id,
        notification_type: parkhub_common::models::NotificationType::BookingConfirmed,
        title: "Booking Confirmed".to_string(),
        message: "Your slot A-12 is booked for 14:00-16:00".to_string(),
        data: None,
        read: false,
        created_at: Utc::now(),
    };

    // Save
    db.save_notification(&notif).await.unwrap();

    // List by user
    let list = db
        .list_notifications_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].title, "Booking Confirmed");
    assert!(!list[0].read);

    // Mark read
    let marked = db
        .mark_notification_read(&notif.id.to_string())
        .await
        .unwrap();
    assert!(marked);

    // Verify read=true
    let list = db
        .list_notifications_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert!(list[0].read);
}

// ═══════════════════════════════════════════════════════════════════════════
// ANNOUNCEMENT OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_announcement_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let ann = Announcement {
        id: Uuid::new_v4(),
        title: "Maintenance Notice".to_string(),
        message: "Level B2 closed for repairs on Saturday".to_string(),
        severity: parkhub_common::models::AnnouncementSeverity::Warning,
        active: true,
        created_by: Some(Uuid::new_v4()),
        expires_at: Some(Utc::now() + chrono::Duration::days(7)),
        created_at: Utc::now(),
    };

    // Save
    db.save_announcement(&ann).await.unwrap();

    // List
    let all = db.list_announcements().await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].title, "Maintenance Notice");
    assert_eq!(
        all[0].severity,
        parkhub_common::models::AnnouncementSeverity::Warning
    );

    // Delete
    let deleted = db.delete_announcement(&ann.id.to_string()).await.unwrap();
    assert!(deleted);
    assert!(db.list_announcements().await.unwrap().is_empty());

    // Delete non-existent
    let nope = db
        .delete_announcement(&Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!nope);
}

// ═══════════════════════════════════════════════════════════════════════════
// ABSENCE OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_absence_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let absence = Absence {
        id: Uuid::new_v4(),
        user_id,
        absence_type: parkhub_common::models::AbsenceType::Homeoffice,
        start_date: "2026-03-20".to_string(),
        end_date: "2026-03-20".to_string(),
        note: Some("Working from home".to_string()),
        source: "manual".to_string(),
        created_at: Utc::now(),
    };

    // Create
    db.save_absence(&absence).await.unwrap();

    // List by user
    let list = db
        .list_absences_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].start_date, "2026-03-20");

    // Delete
    let deleted = db.delete_absence(&absence.id.to_string()).await.unwrap();
    assert!(deleted);
    assert!(
        db.list_absences_by_user(&user_id.to_string())
            .await
            .unwrap()
            .is_empty()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// CREDIT TRANSACTION OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_credit_transaction_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let tx1 = parkhub_common::models::CreditTransaction {
        id: Uuid::new_v4(),
        user_id,
        booking_id: Some(Uuid::new_v4()),
        amount: -2,
        transaction_type: parkhub_common::models::CreditTransactionType::Deduction,
        description: Some("Booking slot A-5".to_string()),
        granted_by: None,
        created_at: Utc::now() - chrono::Duration::minutes(10),
    };
    let tx2 = parkhub_common::models::CreditTransaction {
        id: Uuid::new_v4(),
        user_id,
        booking_id: None,
        amount: 40,
        transaction_type: parkhub_common::models::CreditTransactionType::MonthlyRefill,
        description: Some("Monthly refill".to_string()),
        granted_by: None,
        created_at: Utc::now(),
    };

    db.save_credit_transaction(&tx1).await.unwrap();
    db.save_credit_transaction(&tx2).await.unwrap();

    let list = db.list_credit_transactions_for_user(user_id).await.unwrap();
    assert_eq!(list.len(), 2);
    // Sorted newest first
    assert_eq!(
        list[0].transaction_type,
        parkhub_common::models::CreditTransactionType::MonthlyRefill
    );
    assert_eq!(
        list[1].transaction_type,
        parkhub_common::models::CreditTransactionType::Deduction
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// SLOT OPERATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_slot_batch_save() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();

    let slots: Vec<ParkingSlot> = (1..=5).map(|n| make_slot(lot_id, floor_id, n)).collect();

    db.save_parking_slots_batch(&slots).await.unwrap();

    let by_lot = db.list_slots_by_lot(&lot_id.to_string()).await.unwrap();
    assert_eq!(by_lot.len(), 5);

    // Each slot accessible by ID
    for slot in &slots {
        let fetched = db
            .get_parking_slot(&slot.id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.lot_id, lot_id);
    }
}

#[tokio::test]
async fn test_slot_status_update() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let lot_id = Uuid::new_v4();
    let floor_id = Uuid::new_v4();
    let slot = make_slot(lot_id, floor_id, 1);

    db.save_parking_slot(&slot).await.unwrap();
    let fetched = db
        .get_parking_slot(&slot.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.status, SlotStatus::Available);

    // Update status
    let updated = db
        .update_slot_status(&slot.id.to_string(), SlotStatus::Occupied)
        .await
        .unwrap();
    assert!(updated);

    let after = db
        .get_parking_slot(&slot.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.status, SlotStatus::Occupied);

    // Update non-existent slot returns false
    let nope = db
        .update_slot_status(&Uuid::new_v4().to_string(), SlotStatus::Maintenance)
        .await
        .unwrap();
    assert!(!nope);
}

// ═══════════════════════════════════════════════════════════════════════════
// STATS
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_stats_after_data() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    // Users
    let u1 = make_user("alice", "alice@stats.com");
    let u2 = make_user("bob", "bob@stats.com");
    db.save_user(&u1).await.unwrap();
    db.save_user(&u2).await.unwrap();

    // Parking lot
    let lot = make_parking_lot();
    db.save_parking_lot(&lot).await.unwrap();

    // Slots
    let floor_id = Uuid::new_v4();
    let s1 = make_slot(lot.id, floor_id, 1);
    let s2 = make_slot(lot.id, floor_id, 2);
    let s3 = make_slot(lot.id, floor_id, 3);
    db.save_parking_slots_batch(&[s1, s2, s3]).await.unwrap();

    // Vehicle + Bookings
    let v = make_vehicle(u1.id, "M-ST 1111");
    db.save_vehicle(&v).await.unwrap();
    let b1 = make_booking(u1.id, lot.id, &v);
    let b2 = make_booking(u1.id, lot.id, &v);
    db.save_booking(&b1).await.unwrap();
    db.save_booking(&b2).await.unwrap();

    // Session
    let session = Session::new(u1.id, 24, "alice", "user");
    db.save_session("stats_tok", &session).await.unwrap();

    let stats = db.stats().await.unwrap();
    assert_eq!(stats.users, 2);
    assert_eq!(stats.parking_lots, 1);
    assert_eq!(stats.slots, 3);
    assert_eq!(stats.bookings, 2);
    assert_eq!(stats.vehicles, 1);
    assert_eq!(stats.sessions, 1);
}

#[tokio::test]
async fn test_database_stats_empty() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let stats = db.stats().await.unwrap();
    assert_eq!(stats.users, 0);
    assert_eq!(stats.bookings, 0);
    assert_eq!(stats.parking_lots, 0);
    assert_eq!(stats.slots, 0);
    assert_eq!(stats.sessions, 0);
    assert_eq!(stats.vehicles, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// FAVORITES CRUD
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_favorites_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let slot_a = Uuid::new_v4();
    let slot_b = Uuid::new_v4();
    let lot_id = Uuid::new_v4();

    // No favorites initially
    let favs = db
        .list_favorites_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert!(favs.is_empty());

    // Add two favorites
    let fav1 = Favorite {
        user_id,
        slot_id: slot_a,
        lot_id,
        created_at: Utc::now(),
    };
    let fav2 = Favorite {
        user_id,
        slot_id: slot_b,
        lot_id,
        created_at: Utc::now(),
    };

    db.save_favorite(&fav1).await.unwrap();
    db.save_favorite(&fav2).await.unwrap();

    // List returns both
    let favs = db
        .list_favorites_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert_eq!(favs.len(), 2);

    // Delete one
    let deleted = db
        .delete_favorite(&user_id.to_string(), &slot_a.to_string())
        .await
        .unwrap();
    assert!(deleted);

    // Only one remains
    let favs = db
        .list_favorites_by_user(&user_id.to_string())
        .await
        .unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].slot_id, slot_b);

    // Delete non-existent returns false
    let nope = db
        .delete_favorite(&user_id.to_string(), &Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!nope);
}

#[tokio::test]
async fn test_favorites_isolation_between_users() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    let slot_id = Uuid::new_v4();
    let lot_id = Uuid::new_v4();

    // Both users favorite the same slot
    let fav_a = Favorite {
        user_id: user_a,
        slot_id,
        lot_id,
        created_at: Utc::now(),
    };
    let fav_b = Favorite {
        user_id: user_b,
        slot_id,
        lot_id,
        created_at: Utc::now(),
    };

    db.save_favorite(&fav_a).await.unwrap();
    db.save_favorite(&fav_b).await.unwrap();

    // Each user sees only their own
    let a_favs = db
        .list_favorites_by_user(&user_a.to_string())
        .await
        .unwrap();
    assert_eq!(a_favs.len(), 1);
    assert_eq!(a_favs[0].user_id, user_a);

    let b_favs = db
        .list_favorites_by_user(&user_b.to_string())
        .await
        .unwrap();
    assert_eq!(b_favs.len(), 1);
    assert_eq!(b_favs[0].user_id, user_b);

    // Delete user_a's favorite doesn't affect user_b
    db.delete_favorite(&user_a.to_string(), &slot_id.to_string())
        .await
        .unwrap();
    assert!(
        db.list_favorites_by_user(&user_a.to_string())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.list_favorites_by_user(&user_b.to_string())
            .await
            .unwrap()
            .len(),
        1
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// GDPR ANONYMIZATION
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_gdpr_anonymize_user() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user = make_user("alice", "alice@example.com");
    db.save_user(&user).await.unwrap();

    // Add a vehicle
    let vehicle = make_vehicle(user.id, "M-AB 1234");
    db.save_vehicle(&vehicle).await.unwrap();

    // Add a booking
    let lot_id = Uuid::new_v4();
    let booking = make_booking(user.id, lot_id, &vehicle);
    db.save_booking(&booking).await.unwrap();

    // Anonymize
    let result = db.anonymize_user(&user.id.to_string()).await.unwrap();
    assert!(result);

    // User still exists but is anonymized
    let anon = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
    assert_eq!(anon.name, "[Deleted User]");
    assert!(anon.username.starts_with("deleted-"));
    assert!(anon.email.ends_with("@deleted.invalid"));

    // Old username/email lookups return None
    assert!(db.get_user_by_username("alice").await.unwrap().is_none());
    assert!(
        db.get_user_by_email("alice@example.com")
            .await
            .unwrap()
            .is_none()
    );

    // New anonymized username lookup works
    let by_name = db.get_user_by_username(&anon.username).await.unwrap();
    assert!(by_name.is_some());

    // Vehicle is deleted
    assert!(
        db.get_vehicle(&vehicle.id.to_string())
            .await
            .unwrap()
            .is_none()
    );

    // Booking still exists but license plate is scrubbed
    let scrubbed = db
        .get_booking(&booking.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(scrubbed.vehicle.license_plate, "[DELETED]");
}

#[tokio::test]
async fn test_gdpr_anonymize_nonexistent_user() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let result = db
        .anonymize_user(&Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(!result);
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION REFRESH TOKEN LOOKUP
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_get_session_by_refresh_token() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let session = Session::new(user_id, 24, "alice", "user");
    let refresh_token = session.refresh_token.clone();
    let access_token = "access_abc";

    db.save_session(access_token, &session).await.unwrap();

    // Lookup by refresh token
    let (found_access, found_session) = db
        .get_session_by_refresh_token(&refresh_token)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_access, access_token);
    assert_eq!(found_session.user_id, user_id);
    assert_eq!(found_session.username, "alice");

    // Non-existent refresh token
    let missing = db
        .get_session_by_refresh_token("nonexistent")
        .await
        .unwrap();
    assert!(missing.is_none());
}

#[tokio::test]
async fn test_get_session_by_refresh_token_expired() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let user_id = Uuid::new_v4();
    let mut session = Session::new(user_id, 1, "expired", "user");
    let refresh_token = session.refresh_token.clone();
    session.expires_at = Utc::now() - chrono::Duration::hours(1);

    db.save_session("expired_tok", &session).await.unwrap();

    // Expired session should return None
    let result = db
        .get_session_by_refresh_token(&refresh_token)
        .await
        .unwrap();
    assert!(result.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// CLEAR ALL DATA — verifies settings preserved
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_clear_all_data_preserves_settings() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    // Add some data
    let user = make_user("alice", "alice@test.com");
    db.save_user(&user).await.unwrap();

    // Set a custom setting
    db.set_setting("custom_key", "custom_value").await.unwrap();
    db.mark_setup_completed().await.unwrap();

    // Clear
    db.clear_all_data().await.unwrap();

    // Users gone
    assert!(db.list_users().await.unwrap().is_empty());

    // Settings preserved
    assert_eq!(
        db.get_setting("custom_key").await.unwrap(),
        Some("custom_value".to_string())
    );
    assert!(!db.is_fresh().await.unwrap());
}

#[tokio::test]
async fn test_clear_all_data_sessions_and_bookings() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    // Sessions
    let session = Session::new(Uuid::new_v4(), 24, "test", "user");
    db.save_session("tok1", &session).await.unwrap();

    // Bookings
    let user = make_user("alice", "alice@test.com");
    let vehicle = make_vehicle(user.id, "X-1");
    let booking = make_booking(user.id, Uuid::new_v4(), &vehicle);
    db.save_user(&user).await.unwrap();
    db.save_booking(&booking).await.unwrap();

    db.clear_all_data().await.unwrap();

    assert!(db.get_session("tok1").await.unwrap().is_none());
    assert!(db.list_bookings().await.unwrap().is_empty());
    assert!(db.list_users().await.unwrap().is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// ANNOUNCEMENT — additional coverage
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_announcement_multiple_and_order() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let a1 = Announcement {
        id: Uuid::new_v4(),
        title: "First".to_string(),
        message: "First announcement".to_string(),
        severity: parkhub_common::models::AnnouncementSeverity::Info,
        active: true,
        created_by: None,
        expires_at: None,
        created_at: Utc::now(),
    };
    let a2 = Announcement {
        id: Uuid::new_v4(),
        title: "Second".to_string(),
        message: "Second announcement".to_string(),
        severity: parkhub_common::models::AnnouncementSeverity::Error,
        active: false,
        created_by: Some(Uuid::new_v4()),
        expires_at: Some(Utc::now() + chrono::Duration::days(30)),
        created_at: Utc::now(),
    };

    db.save_announcement(&a1).await.unwrap();
    db.save_announcement(&a2).await.unwrap();

    let all = db.list_announcements().await.unwrap();
    assert_eq!(all.len(), 2);

    let titles: Vec<&str> = all.iter().map(|a| a.title.as_str()).collect();
    assert!(titles.contains(&"First"));
    assert!(titles.contains(&"Second"));

    // Verify severity is preserved
    let critical = all.iter().find(|a| a.title == "Second").unwrap();
    assert_eq!(
        critical.severity,
        parkhub_common::models::AnnouncementSeverity::Error
    );
    assert!(!critical.active);
}

// ═══════════════════════════════════════════════════════════════════════════
// PARKING LOT CRUD
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_parking_lot_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), false)).unwrap();

    let lot = make_parking_lot();

    // Save
    db.save_parking_lot(&lot).await.unwrap();

    // Get
    let fetched = db
        .get_parking_lot(&lot.id.to_string())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.name, "Test Lot");
    assert_eq!(fetched.address, "123 Test St");

    // List
    let all = db.list_parking_lots().await.unwrap();
    assert_eq!(all.len(), 1);

    // Non-existent
    let missing = db
        .get_parking_lot(&Uuid::new_v4().to_string())
        .await
        .unwrap();
    assert!(missing.is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// SESSION — constructor invariants
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_new_fields() {
    let user_id = Uuid::new_v4();
    let session = Session::new(user_id, 48, "testuser", "admin");

    assert_eq!(session.user_id, user_id);
    assert_eq!(session.username, "testuser");
    assert_eq!(session.role, "admin");
    assert!(session.refresh_token.starts_with("rt_"));
    // Refresh token has 64 hex chars after prefix
    assert_eq!(session.refresh_token.len(), 3 + 64);
    assert!(!session.is_expired());
    assert!(session.expires_at > session.created_at);
}

#[test]
fn test_session_unique_refresh_tokens() {
    let uid = Uuid::new_v4();
    let s1 = Session::new(uid, 1, "u", "r");
    let s2 = Session::new(uid, 1, "u", "r");
    assert_ne!(
        s1.refresh_token, s2.refresh_token,
        "Each session must have a unique refresh token"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// ENCRYPTED DATABASE — data roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_encrypted_data_roundtrip() {
    let dir = tempdir().unwrap();
    let db = Database::open(&test_config(dir.path().to_path_buf(), true)).unwrap();
    assert!(db.is_encrypted());

    // Save and retrieve a user through encryption
    let user = make_user("encrypted_alice", "encrypted@test.com");
    db.save_user(&user).await.unwrap();

    let fetched = db.get_user(&user.id.to_string()).await.unwrap().unwrap();
    assert_eq!(fetched.username, "encrypted_alice");
    assert_eq!(fetched.email, "encrypted@test.com");
}
