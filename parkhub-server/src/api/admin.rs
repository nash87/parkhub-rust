//! Admin handlers: user management, booking management, settings, features,
//! impressum, announcements, guest bookings, stats, reports, heatmap.
//!
//! Shared types used by other modules (`AdminUserResponse`, `AdminBookingResponse`)
//! are defined here and re-exported.
//!
//! TODO: Move these handlers from mod.rs into this module:
//! - `admin_list_users`
//! - `admin_update_user_role`
//! - `admin_update_user_status`
//! - `admin_delete_user`
//! - `admin_list_bookings`
//! - `admin_get_settings` / `admin_update_settings`
//! - `admin_get_features` / `admin_update_features`
//! - `get_impressum` / `get_impressum_admin` / `update_impressum`
//! - `admin_list_announcements` / `admin_create_announcement` / `admin_update_announcement` / `admin_delete_announcement`
//! - `admin_list_guest_bookings` / `admin_cancel_guest_booking`
//! - `admin_stats` / `admin_reports` / `admin_heatmap`

use chrono::Utc;
use serde::Serialize;

use parkhub_common::User;

// ─────────────────────────────────────────────────────────────────────────────
// Shared types (used by credits.rs and mod.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Response type for admin user listing (includes status field)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AdminUserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub credits_balance: i32,
    pub credits_monthly_quota: i32,
    pub is_active: bool,
    pub created_at: chrono::DateTime<Utc>,
}

impl From<&User> for AdminUserResponse {
    fn from(u: &User) -> Self {
        Self {
            id: u.id.to_string(),
            username: u.username.clone(),
            email: u.email.clone(),
            name: u.name.clone(),
            role: format!("{:?}", u.role).to_lowercase(),
            status: if u.is_active {
                "active".to_string()
            } else {
                "disabled".to_string()
            },
            credits_balance: u.credits_balance,
            credits_monthly_quota: u.credits_monthly_quota,
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkhub_common::models::UserPreferences;
    use parkhub_common::UserRole;
    use uuid::Uuid;

    fn make_test_user(role: UserRole, is_active: bool) -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password_hash: "hash".to_string(),
            role,
            is_active,
            phone: None,
            picture: None,
            preferences: UserPreferences {
                language: "en".to_string(),
                theme: "system".to_string(),
                notifications_enabled: true,
                email_reminders: false,
                default_duration_minutes: None,
                favorite_slots: Vec::new(),
            },
            credits_balance: 5,
            credits_monthly_quota: 10,
            credits_last_refilled: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            tenant_id: None,
            accessibility_needs: None,
        }
    }

    #[test]
    fn test_admin_user_response_from_active_admin() {
        let user = make_test_user(UserRole::Admin, true);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.username, "testuser");
        assert_eq!(resp.email, "test@example.com");
        assert_eq!(resp.role, "admin");
        assert_eq!(resp.status, "active");
        assert!(resp.is_active);
        assert_eq!(resp.credits_balance, 5);
        assert_eq!(resp.credits_monthly_quota, 10);
    }

    #[test]
    fn test_admin_user_response_from_disabled_user() {
        let user = make_test_user(UserRole::User, false);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.role, "user");
        assert_eq!(resp.status, "disabled");
        assert!(!resp.is_active);
    }

    #[test]
    fn test_admin_user_response_from_superadmin() {
        let user = make_test_user(UserRole::SuperAdmin, true);
        let resp = AdminUserResponse::from(&user);
        assert_eq!(resp.role, "superadmin");
        assert_eq!(resp.status, "active");
    }

    #[test]
    fn test_admin_user_response_serialize() {
        let user = make_test_user(UserRole::Admin, true);
        let resp = AdminUserResponse::from(&user);
        let json = serde_json::to_string(&resp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["username"], "testuser");
        assert_eq!(value["role"], "admin");
        assert_eq!(value["status"], "active");
        assert_eq!(value["credits_balance"], 5);
        assert_eq!(value["is_active"], true);
    }

    #[test]
    fn test_admin_user_response_id_is_uuid_string() {
        let user = make_test_user(UserRole::User, true);
        let resp = AdminUserResponse::from(&user);
        // ID should be parseable back to UUID
        assert!(Uuid::parse_str(&resp.id).is_ok());
    }
}
