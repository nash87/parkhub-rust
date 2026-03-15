//! Admin handlers: user management, booking management, settings, features,
//! impressum, announcements, guest bookings, stats, reports, heatmap.
//!
//! Shared types used by other modules (AdminUserResponse, AdminBookingResponse)
//! are defined here and re-exported.
//!
//! TODO: Move these handlers from mod.rs into this module:
//! - admin_list_users
//! - admin_update_user_role
//! - admin_update_user_status
//! - admin_delete_user
//! - admin_list_bookings
//! - admin_get_settings / admin_update_settings
//! - admin_get_features / admin_update_features
//! - get_impressum / get_impressum_admin / update_impressum
//! - admin_list_announcements / admin_create_announcement / admin_update_announcement / admin_delete_announcement
//! - admin_list_guest_bookings / admin_cancel_guest_booking
//! - admin_stats / admin_reports / admin_heatmap

use chrono::Utc;
use serde::Serialize;

use parkhub_common::User;

// ─────────────────────────────────────────────────────────────────────────────
// Shared types (used by credits.rs and mod.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Response type for admin user listing (includes status field)
#[derive(Debug, Serialize)]
pub(crate) struct AdminUserResponse {
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
