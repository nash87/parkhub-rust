//! `OpenAPI` Documentation
//!
//! Generates `OpenAPI` 3.0 specification and Swagger UI.

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    api::{
        admin::AdminUserResponse,
        credits::AdminGrantCreditsRequest,
        favorites::AddFavoriteRequest,
        push::{PushKeys, SubscribeRequest, SubscriptionResponse, VapidKeyResponse},
        setup::{SetupRequest, SetupStatus},
        webhooks::{CreateWebhookRequest, UpdateWebhookRequest, WebhookResponse},
        zones::CreateZoneRequest,
    },
    error::{ApiError, FieldError},
    health::{ComponentHealth, HealthResponse, HealthStatus, ReadyResponse},
    jwt::TokenPair,
    requests::{
        BookingFiltersParams, ChangePasswordRequest, CreateBookingRequest, CreateParkingLotRequest,
        ExtendBookingRequest, LoginRequest, PaginationParams, RefreshTokenRequest, RegisterRequest,
        UpdateBookingRequest, UpdateParkingLotRequest, UpdatePreferencesRequest,
        UpdateProfileRequest, UpdateQuotaRequest, VehicleRequest,
    },
};

/// `OpenAPI` documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "ParkHub API",
        version = "1.0.0",
        description = "Open-source parking lot management system API. Provides endpoints for user authentication, booking management, parking lot administration, credit system, webhooks, and more.",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(
            name = "ParkHub",
            url = "https://github.com/nash87/parkhub"
        )
    ),
    servers(
        (url = "/", description = "Local server")
    ),
    tags(
        (name = "Authentication", description = "User login, registration, and password management"),
        (name = "Users", description = "User profile and account management"),
        (name = "Bookings", description = "Parking booking lifecycle (create, list, cancel, check-in)"),
        (name = "Lots", description = "Parking lots and slot management"),
        (name = "Zones", description = "Parking lot zone management"),
        (name = "Vehicles", description = "User vehicle registration and management"),
        (name = "Credits", description = "Credit balance, grants, and quota management"),
        (name = "Favorites", description = "User favorite parking slots"),
        (name = "Webhooks", description = "Webhook configuration and event delivery"),
        (name = "Push", description = "Web Push notification subscriptions"),
        (name = "Setup", description = "Initial system setup wizard"),
        (name = "Admin", description = "Administrative endpoints (user management, settings, exports, reports)"),
        (name = "Demo", description = "Public demo mode endpoints"),
        (name = "Health", description = "Health check and readiness probes"),
        (name = "Monitoring", description = "Prometheus metrics"),
        (name = "Public", description = "Unauthenticated public endpoints"),
        (name = "Absences", description = "User absence management (vacation, sick, home office)"),
        (name = "Notifications", description = "In-app notification management"),
        (name = "Waitlist", description = "Parking lot waitlist"),
        (name = "Calendar", description = "Calendar events and iCal export"),
        (name = "Team", description = "Team overview and member status"),
        (name = "Payments", description = "Stripe payment stub (demo mode)"),
        (name = "Translations", description = "Community translation proposals, voting, and overrides")
    ),
    components(
        schemas(
            // Errors
            ApiError,
            FieldError,

            // Auth
            LoginRequest,
            RegisterRequest,
            ChangePasswordRequest,
            RefreshTokenRequest,
            TokenPair,

            // Auth (submodule types)
            crate::api::auth::ForgotPasswordRequest,
            crate::api::auth::ResetPasswordRequest,

            // Bookings
            CreateBookingRequest,
            ExtendBookingRequest,
            UpdateBookingRequest,
            BookingFiltersParams,

            // Vehicles
            VehicleRequest,

            // Users
            UpdateProfileRequest,
            UpdatePreferencesRequest,

            // Admin
            CreateParkingLotRequest,
            UpdateParkingLotRequest,
            crate::api::lots::UpdateLotPricingRequest,
            AdminUserResponse,
            UpdateQuotaRequest,
            crate::api::import::ImportResult,
            crate::api::import::ImportError,

            // Credits
            AdminGrantCreditsRequest,

            // Webhooks
            CreateWebhookRequest,
            UpdateWebhookRequest,
            WebhookResponse,

            // Zones
            CreateZoneRequest,

            // Favorites
            AddFavoriteRequest,

            // Push
            SubscribeRequest,
            PushKeys,
            SubscriptionResponse,
            VapidKeyResponse,

            // Setup
            SetupStatus,
            SetupRequest,

            // Common
            PaginationParams,

            // Health
            HealthResponse,
            HealthStatus,
            ComponentHealth,
            ReadyResponse,

            // Swap Requests
            crate::api::CreateSwapRequestBody,
            crate::api::UpdateSwapRequestBody,

            // Recurring Bookings
            crate::api::CreateRecurringBookingRequest,

            // Guest Bookings
            crate::api::CreateGuestBookingRequest,

            // Announcements
            crate::api::CreateAnnouncementRequest,
            crate::api::UpdateAnnouncementRequest,

            // Admin Settings
            crate::api::AutoReleaseSettingsRequest,
            crate::api::EmailSettingsRequest,
            crate::api::PrivacySettingsRequest,
            crate::api::AdminUpdateUserRequest,

            // Payments
            crate::api::payments::CreatePaymentIntentRequest,
            crate::api::payments::ConfirmPaymentRequest,
            crate::api::payments::PaymentIntentResponse,
            crate::api::payments::PaymentStatusResponse,
            crate::api::payments::StripePaymentStatus,

            // Recommendations
            crate::api::recommendations::SlotRecommendation,
            crate::api::recommendations::RecommendationQuery,

            // Translations
            crate::api::translations::CreateProposalRequest,
            crate::api::translations::VoteRequest,
            crate::api::translations::ReviewRequest,
            parkhub_common::models::TranslationProposal,
            parkhub_common::models::TranslationOverride,
            parkhub_common::models::ProposalStatus,
        )
    ),
    paths(
        // Authentication
        crate::api::auth::login,
        crate::api::auth::register,
        crate::api::auth::refresh_token,
        crate::api::auth::forgot_password,
        crate::api::auth::reset_password,

        // Lots & Slots
        crate::api::lots::list_lots,
        crate::api::lots::create_lot,
        crate::api::lots::get_lot,
        crate::api::lots::update_lot,
        crate::api::lots::delete_lot,
        crate::api::lots::get_lot_slots,
        crate::api::lots::create_slot,
        crate::api::lots::update_slot,
        crate::api::lots::delete_slot,
        crate::api::lots::get_lot_pricing,
        crate::api::lots::update_lot_pricing,

        // Zones
        crate::api::zones::list_zones,
        crate::api::zones::create_zone,
        crate::api::zones::delete_zone,

        // Credits
        crate::api::credits::get_user_credits,
        crate::api::credits::admin_grant_credits,
        crate::api::credits::admin_refill_all_credits,
        crate::api::credits::admin_update_user_quota,

        // Webhooks
        crate::api::webhooks::list_webhooks,
        crate::api::webhooks::create_webhook,
        crate::api::webhooks::update_webhook,
        crate::api::webhooks::delete_webhook,
        crate::api::webhooks::test_webhook,

        // Favorites
        crate::api::favorites::list_favorites,
        crate::api::favorites::add_favorite,
        crate::api::favorites::remove_favorite,

        // Push notifications
        crate::api::push::get_vapid_key,
        crate::api::push::subscribe,
        crate::api::push::unsubscribe,

        // Setup
        crate::api::setup::setup_status,
        crate::api::setup::setup_init,

        // Exports
        crate::api::export::admin_export_users_csv,
        crate::api::export::admin_export_bookings_csv,
        crate::api::export::admin_export_revenue_csv,
        // Import
        crate::api::import::import_users_csv,

        // Health & Discovery (mod.rs)
        crate::api::health_check,
        crate::api::liveness_check,
        crate::api::readiness_check,
        crate::api::handshake,
        crate::api::server_status,

        // Users (mod.rs)
        crate::api::get_current_user,
        crate::api::update_current_user,
        crate::api::get_user,
        crate::api::change_password,
        crate::api::user_stats,
        crate::api::get_user_preferences,
        crate::api::update_user_preferences,
        crate::api::gdpr_export_data,
        crate::api::gdpr_delete_account,

        // Bookings (mod.rs)
        crate::api::list_bookings,
        crate::api::create_booking,
        crate::api::get_booking,
        crate::api::cancel_booking,
        crate::api::get_booking_invoice,
        crate::api::quick_book,
        crate::api::booking_checkin,

        // Vehicles
        crate::api::vehicles::list_vehicles,
        crate::api::vehicles::create_vehicle,
        crate::api::vehicles::update_vehicle,
        crate::api::vehicles::delete_vehicle,
        crate::api::vehicles::upload_vehicle_photo,
        crate::api::vehicles::get_vehicle_photo,
        crate::api::vehicles::vehicle_city_codes,
        crate::api::lot_qr_code,

        // Admin (mod.rs)
        crate::api::admin_list_users,
        crate::api::admin_update_user_role,
        crate::api::admin_update_user_status,
        crate::api::admin_delete_user,
        crate::api::admin_list_bookings,
        crate::api::admin_get_settings,
        crate::api::admin_update_settings,
        crate::api::admin_get_features,
        crate::api::admin_update_features,
        crate::api::admin_stats,
        crate::api::admin_reports,
        crate::api::admin_heatmap,
        crate::api::admin_dashboard_charts,
        crate::api::admin_audit_log,
        crate::api::admin_reset,
        crate::api::get_impressum_admin,
        crate::api::update_impressum,
        crate::api::admin_list_announcements,

        // Public (mod.rs)
        crate::api::get_impressum,
        crate::api::get_features,
        crate::api::get_public_theme,
        crate::api::get_active_announcements,
        crate::api::public_occupancy,
        crate::api::public_display,

        // Absences (mod.rs)
        crate::api::list_absences,
        crate::api::create_absence,
        crate::api::delete_absence,

        // Notifications (mod.rs)
        crate::api::list_notifications,
        crate::api::mark_notification_read,
        crate::api::mark_all_notifications_read,

        // Waitlist (mod.rs)
        crate::api::list_waitlist,
        crate::api::join_waitlist,
        crate::api::leave_waitlist,

        // Calendar (mod.rs)
        crate::api::calendar_events,
        crate::api::user_calendar_ics,

        // Team (mod.rs)
        crate::api::team_today,
        crate::api::team_list,

        // Swap Requests (mod.rs)
        crate::api::list_swap_requests,
        crate::api::create_swap_request,
        crate::api::update_swap_request,

        // Recurring Bookings (mod.rs)
        crate::api::list_recurring_bookings,
        crate::api::create_recurring_booking,
        crate::api::delete_recurring_booking,

        // Guest Bookings (mod.rs)
        crate::api::create_guest_booking,
        crate::api::admin_list_guest_bookings,
        crate::api::admin_cancel_guest_booking,

        // Absences — additional (mod.rs)
        crate::api::list_team_absences,
        crate::api::get_absence_pattern,
        crate::api::save_absence_pattern,

        // Announcements (mod.rs)
        crate::api::admin_create_announcement,
        crate::api::admin_update_announcement,
        crate::api::admin_delete_announcement,

        // Admin — additional settings (mod.rs)
        crate::api::admin_get_use_case,
        crate::api::admin_get_auto_release,
        crate::api::admin_update_auto_release,
        crate::api::admin_get_email_settings,
        crate::api::admin_update_email_settings,
        crate::api::admin_get_privacy,
        crate::api::admin_update_privacy,
        crate::api::admin_update_user,

        // QR Pass
        crate::api::qr::booking_qr_code,

        // Payments (Stripe stub)
        crate::api::payments::create_payment_intent,
        crate::api::payments::confirm_payment,
        crate::api::payments::payment_status,

        // Recommendations
        crate::api::recommendations::get_recommendations,

        // Translations
        crate::api::translations::list_overrides,
        crate::api::translations::list_proposals,
        crate::api::translations::get_proposal,
        crate::api::translations::create_proposal,
        crate::api::translations::vote_on_proposal,
        crate::api::translations::review_proposal,
    )
)]
pub struct ApiDoc;

/// Create Swagger UI router
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}

/// Add `OpenAPI` routes to router
pub fn with_openapi(router: Router) -> Router {
    router.merge(swagger_ui())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().expect("Failed to generate OpenAPI JSON");
        assert!(json.contains("ParkHub API"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_openapi_has_all_tags() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for tag in [
            "Authentication",
            "Users",
            "Bookings",
            "Lots",
            "Zones",
            "Vehicles",
            "Credits",
            "Webhooks",
            "Favorites",
            "Push",
            "Setup",
            "Admin",
            "Health",
            "Public",
            "Absences",
            "Notifications",
            "Waitlist",
            "Calendar",
            "Team",
        ] {
            assert!(json.contains(tag), "Missing tag: {tag}");
        }
    }

    #[test]
    fn test_openapi_has_auth_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/auth/login",
            "/auth/register",
            "/auth/refresh",
            "/auth/forgot-password",
            "/auth/reset-password",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_lot_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/lots", "/lots/{id}", "/lots/{lot_id}/slots"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_webhook_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/webhooks", "/webhooks/{id}", "/webhooks/{id}/test"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_zone_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        assert!(json.contains("/lots/{lot_id}/zones"), "Missing zones path");
    }

    #[test]
    fn test_openapi_has_push_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/push/vapid-key", "/push/subscribe", "/push/unsubscribe"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_setup_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/setup/status", "/setup"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_export_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/admin/export/users",
            "/api/v1/admin/export/bookings",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_user_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/users/me",
            "/api/v1/users/me/password",
            "/api/v1/users/me/export",
            "/api/v1/users/me/delete",
            "/api/v1/user/stats",
            "/api/v1/user/preferences",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_booking_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/bookings",
            "/api/v1/bookings/{id}",
            "/api/v1/bookings/{id}/checkin",
            "/api/v1/bookings/quick",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_vehicle_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/vehicles",
            "/api/v1/vehicles/{id}",
            "/api/v1/vehicles/{id}/photo",
            "/api/v1/vehicles/city-codes",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_admin_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/admin/users",
            "/api/v1/admin/bookings",
            "/api/v1/admin/stats",
            "/api/v1/admin/settings",
            "/api/v1/admin/audit-log",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_health_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/health", "/health/live", "/health/ready", "/status"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_schemas() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for schema in [
            "LoginRequest",
            "RegisterRequest",
            "CreateParkingLotRequest",
            "CreateWebhookRequest",
            "WebhookResponse",
            "AdminUserResponse",
            "SetupRequest",
            "SubscribeRequest",
            "AddFavoriteRequest",
            "CreateZoneRequest",
            "VapidKeyResponse",
        ] {
            assert!(json.contains(schema), "Missing schema: {schema}");
        }
    }

    #[test]
    fn test_openapi_has_swap_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/swap-requests",
            "/api/v1/bookings/{id}/swap-request",
            "/api/v1/swap-requests/{id}",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_recurring_booking_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        {
            let path = "/api/v1/recurring-bookings";
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_guest_booking_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/api/v1/bookings/guest", "/api/v1/admin/guest-bookings"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_announcement_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        {
            let path = "/api/v1/admin/announcements";
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_admin_settings_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/admin/settings/use-case",
            "/api/v1/admin/settings/auto-release",
            "/api/v1/admin/settings/email",
            "/api/v1/admin/privacy",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_absence_pattern_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/api/v1/absences/team", "/api/v1/absences/pattern"] {
            assert!(json.contains(path), "Missing path: {path}");
        }
    }

    #[test]
    fn test_openapi_has_new_schemas() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for schema in [
            "CreateSwapRequestBody",
            "UpdateSwapRequestBody",
            "CreateRecurringBookingRequest",
            "CreateGuestBookingRequest",
            "CreateAnnouncementRequest",
            "UpdateAnnouncementRequest",
            "AutoReleaseSettingsRequest",
            "EmailSettingsRequest",
            "PrivacySettingsRequest",
            "AdminUpdateUserRequest",
        ] {
            assert!(json.contains(schema), "Missing schema: {schema}");
        }
    }

    #[test]
    fn test_openapi_total_paths_count() {
        let doc = ApiDoc::openapi();
        let paths = doc.paths.paths.len();
        assert!(
            paths >= 80,
            "Expected at least 80 documented paths, got {paths}"
        );
    }
}
