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
        (name = "Translations", description = "Community translation proposals, voting, and overrides"),
        (name = "Security", description = "Password policy, login history, and active session management"),
        (name = "2FA", description = "Two-factor authentication setup, verification, and login flow"),
        (name = "API Keys", description = "Personal API key creation and revocation"),
        (name = "OAuth", description = "Third-party OAuth sign-in (Google, GitHub)"),
        (name = "RBAC", description = "Role-based access control: roles, permissions, and assignments"),
        (name = "Branding", description = "Tenant branding — colors, fonts, and logo upload"),
        (name = "Maintenance", description = "Scheduled lot/slot maintenance windows"),
        (name = "Billing", description = "Cost-center and department billing rollups"),
        (name = "Absence Approval", description = "Manager approval workflow for absence requests"),
        (name = "Visitors", description = "Visitor registration, check-in, and admin oversight"),
        (name = "Accessible", description = "Accessibility — accessible slot listing, toggling, user needs"),
        (name = "Documentation", description = "Self-hosted API documentation (Swagger UI, OpenAPI JSON, Postman)"),
        (name = "EV Charging", description = "Electric vehicle charging station management"),
        (name = "Notification Center", description = "Persistent in-app notification center (list, unread count, mark all read)"),
        (name = "Parking Pass", description = "Digital parking passes with QR verification"),
        (name = "Calendar Drag", description = "Calendar drag-and-drop booking reschedule"),
        (name = "Dynamic Pricing", description = "Time-of-day / demand-based dynamic pricing rules"),
        (name = "Operating Hours", description = "Per-lot operating hours and closures"),
        (name = "Parking Zones", description = "Zone-level pricing and price lookup"),
        (name = "Mobile Booking", description = "Mobile-optimised quick actions (nearby lots, quick book, active booking)"),
        (name = "Map", description = "Map markers and admin lot location editing"),
        (name = "Admin Widgets", description = "Admin dashboard widget layout + data"),
        (name = "Stripe", description = "Stripe payments (checkout, webhook, history, config)"),
        (name = "Audit Export", description = "Enhanced audit-log export with signed download tokens"),
        (name = "Invoices", description = "Per-booking invoice PDF rendering")
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
            crate::api::swap::CreateSwapRequestBody,
            crate::api::swap::UpdateSwapRequestBody,

            // Recurring Bookings
            crate::api::recurring::CreateRecurringBookingRequest,

            // Guest Bookings
            crate::api::guest::CreateGuestBookingRequest,

            // Announcements
            crate::api::announcements::CreateAnnouncementRequest,
            crate::api::announcements::UpdateAnnouncementRequest,

            // Admin Settings
            crate::api::admin_handlers::AutoReleaseSettingsRequest,
            crate::api::admin_handlers::EmailSettingsRequest,
            crate::api::admin_handlers::PrivacySettingsRequest,
            crate::api::admin_handlers::AdminUpdateUserRequest,

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

            // Modules registry (T-1720 — admin Modules Dashboard + Command Palette)
            crate::api::modules::ModuleInfo,
            crate::api::modules::ModuleCategory,
            crate::api::modules::ListModulesResponse,
            crate::api::modules::UpdateModuleRequest,
            // T-1720 v3 — per-module JSON Schema config editor
            crate::api::modules::ConfigSchema,
            crate::api::modules::ModuleConfig,
            crate::api::modules::UpdateModuleConfigRequest,

            // T-1739 pass 1 — security/2FA/API-keys/password-policy
            crate::api::security::TwoFactorLoginRequest,
            crate::api::security::TwoFactorVerifyRequest,
            crate::api::security::TwoFactorDisableRequest,
            crate::api::security::CreateApiKeyRequest,
            crate::api::security::PasswordPolicy,

            // T-1739 pass 1 — RBAC
            crate::api::rbac::CreateRoleRequest,
            crate::api::rbac::UpdateRoleRequest,
            crate::api::rbac::AssignRolesRequest,

            // T-1739 pass 1 — Zones (update)
            crate::api::zones::UpdateZoneRequest,

            // T-1739 pass 2 — Admin bulk ops, booking/notification prefs, data management
            crate::api::admin_ext::BulkUserUpdateRequest,
            crate::api::admin_ext::BulkDeleteRequest,
            crate::api::admin_ext::BookingPolicies,
            crate::api::admin_ext::NotificationPreferences,

            // T-1739 pass 2 — Dynamic pricing
            crate::api::dynamic_pricing::UpdateDynamicPricingRequest,

            // T-1739 pass 2 — Import (iCal)
            crate::api::import::IcalImportResult,

            // T-1739 pass 2 — Map / Parking zones
            crate::api::map::SetLocationRequest,
            crate::api::parking_zones::SetZonePricingRequest,

            // T-1739 pass 2 — Stripe / Checkout
            crate::api::stripe::CreateCheckoutRequest,
            crate::api::stripe::WebhookEvent,
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
        crate::api::system::health_check,
        crate::api::system::liveness_check,
        crate::api::system::readiness_check,
        crate::api::system::v1_health,
        crate::api::system::v1_health_live,
        crate::api::system::v1_health_ready,
        crate::api::system::v1_health_info,
        crate::api::system::v1_discover,
        crate::api::system::handshake,
        crate::api::system::server_status,

        // Users (mod.rs)
        crate::api::users::get_current_user,
        crate::api::users::update_current_user,
        crate::api::users::get_user,
        crate::api::users::change_password,
        crate::api::users::user_stats,
        crate::api::users::get_user_preferences,
        crate::api::users::update_user_preferences,
        crate::api::users::gdpr_export_data,
        crate::api::users::gdpr_delete_account,
        crate::api::admin_ext::get_design_theme_preference,
        crate::api::admin_ext::update_design_theme_preference,

        // Bookings (bookings.rs)
        crate::api::bookings::list_bookings,
        crate::api::bookings::create_booking,
        crate::api::bookings::get_booking,
        crate::api::bookings::cancel_booking,
        crate::api::bookings::get_booking_invoice,
        crate::api::bookings::quick_book,
        crate::api::bookings::booking_checkin,

        // Vehicles
        crate::api::vehicles::list_vehicles,
        crate::api::vehicles::create_vehicle,
        crate::api::vehicles::update_vehicle,
        crate::api::vehicles::delete_vehicle,
        crate::api::vehicles::upload_vehicle_photo,
        crate::api::vehicles::get_vehicle_photo,
        crate::api::vehicles::vehicle_city_codes,
        crate::api::lots_ext::lot_qr_code,

        // Admin (mod.rs)
        crate::api::admin_handlers::admin_list_users,
        crate::api::admin_handlers::admin_update_user_role,
        crate::api::admin_handlers::admin_update_user_status,
        crate::api::admin_handlers::admin_delete_user,
        crate::api::admin_handlers::admin_list_bookings,
        crate::api::settings::admin_get_settings,
        crate::api::settings::admin_update_settings,
        crate::api::settings::admin_get_features,
        crate::api::settings::admin_update_features,
        crate::api::admin_handlers::admin_stats,
        crate::api::admin_handlers::admin_reports,
        crate::api::admin_handlers::admin_heatmap,
        crate::api::lots_ext::admin_dashboard_charts,
        crate::api::admin_handlers::admin_audit_log,
        crate::api::admin_handlers::admin_audit_log_export,
        crate::api::admin_handlers::admin_reset,
        crate::api::misc::get_impressum_admin,
        crate::api::misc::update_impressum,
        crate::api::announcements::admin_list_announcements,

        // Public (mod.rs)
        crate::api::misc::get_impressum,
        crate::api::settings::get_features,
        crate::api::settings::get_public_theme,
        crate::api::announcements::get_active_announcements,
        crate::api::misc::public_occupancy,
        crate::api::misc::public_display,

        // Modules registry — enriched metadata for admin Modules Dashboard
        crate::api::modules::list_modules,
        crate::api::modules::get_module,
        crate::api::modules::patch_admin_module,
        // T-1720 v3 — per-module JSON Schema config editor
        crate::api::modules::get_module_config,
        crate::api::modules::patch_module_config,

        // Absences
        crate::api::absences::list_absences,
        crate::api::absences::create_absence,
        crate::api::absences::delete_absence,

        // Notifications
        crate::api::notifications::list_notifications,
        crate::api::notifications::mark_notification_read,
        crate::api::notifications::mark_all_notifications_read,

        // Waitlist
        crate::api::waitlist::list_waitlist,
        crate::api::waitlist::join_waitlist,
        crate::api::waitlist::leave_waitlist,

        // Calendar
        crate::api::calendar::calendar_events,
        crate::api::calendar::user_calendar_ics,

        // Team
        crate::api::team::team_today,
        crate::api::team::team_list,

        // Swap Requests
        crate::api::swap::list_swap_requests,
        crate::api::swap::create_swap_request,
        crate::api::swap::update_swap_request,

        // Recurring Bookings
        crate::api::recurring::list_recurring_bookings,
        crate::api::recurring::create_recurring_booking,
        crate::api::recurring::delete_recurring_booking,

        // Guest Bookings
        crate::api::guest::create_guest_booking,
        crate::api::guest::admin_list_guest_bookings,
        crate::api::guest::admin_cancel_guest_booking,

        // Absences — additional
        crate::api::absences::list_team_absences,
        crate::api::absences::get_absence_pattern,
        crate::api::absences::save_absence_pattern,

        // Announcements
        crate::api::announcements::admin_create_announcement,
        crate::api::announcements::admin_update_announcement,
        crate::api::announcements::admin_delete_announcement,

        // Admin — additional settings
        crate::api::settings::admin_get_use_case,
        crate::api::admin_handlers::admin_get_auto_release,
        crate::api::admin_handlers::admin_update_auto_release,
        crate::api::admin_handlers::admin_get_email_settings,
        crate::api::admin_handlers::admin_update_email_settings,
        crate::api::admin_handlers::admin_get_privacy,
        crate::api::admin_handlers::admin_update_privacy,
        crate::api::admin_handlers::admin_update_user,

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

        // ───── T-1739 pass 1 ─────
        // Auth — session lifecycle
        crate::api::auth::logout,

        // 2FA (Authentication)
        crate::api::security::two_factor_login,
        crate::api::security::two_factor_setup,
        crate::api::security::two_factor_verify,
        crate::api::security::two_factor_disable,
        crate::api::security::two_factor_status,

        // Security — password policy, login history, sessions, API keys
        crate::api::security::get_password_policy,
        crate::api::security::update_password_policy,
        crate::api::security::get_login_history,
        crate::api::security::admin_get_login_history,
        crate::api::security::list_sessions,
        crate::api::security::revoke_session,
        crate::api::security::create_api_key,
        crate::api::security::list_api_keys,
        crate::api::security::revoke_api_key,

        // OAuth
        crate::api::oauth::oauth_providers,
        crate::api::oauth::oauth_google_redirect,
        crate::api::oauth::oauth_google_callback,
        crate::api::oauth::oauth_github_redirect,
        crate::api::oauth::oauth_github_callback,

        // RBAC
        crate::api::rbac::list_roles,
        crate::api::rbac::create_role,
        crate::api::rbac::update_role,
        crate::api::rbac::delete_role,
        crate::api::rbac::get_user_roles,
        crate::api::rbac::assign_user_roles,

        // Branding
        crate::api::branding::admin_get_branding,
        crate::api::branding::admin_update_branding,
        crate::api::branding::admin_upload_logo,
        crate::api::branding::get_branding_logo,

        // Maintenance
        crate::api::maintenance::create_maintenance,
        crate::api::maintenance::list_maintenance,
        crate::api::maintenance::update_maintenance,
        crate::api::maintenance::delete_maintenance,
        crate::api::maintenance::active_maintenance,

        // Billing (cost center / department / export / allocate)
        crate::api::billing::billing_by_cost_center,
        crate::api::billing::billing_by_department,
        crate::api::billing::billing_export_csv,
        crate::api::billing::billing_allocate,

        // Absence Approval workflow
        crate::api::absence_approval::submit_absence_request,
        crate::api::absence_approval::list_pending_absences,
        crate::api::absence_approval::approve_absence,
        crate::api::absence_approval::reject_absence,
        crate::api::absence_approval::my_absence_requests,

        // Visitors
        crate::api::visitors::register_visitor,
        crate::api::visitors::list_my_visitors,
        crate::api::visitors::admin_list_visitors,
        crate::api::visitors::check_in_visitor,
        crate::api::visitors::cancel_visitor,

        // Bookings / Absences / Zones — missing CRUD verbs
        crate::api::bookings::update_booking,
        crate::api::absences::update_absence,
        crate::api::zones::update_zone,

        // Calendar — iCal feed endpoints + token issuance
        crate::api::calendar::calendar_ical_authenticated,
        crate::api::calendar::calendar_ical_by_token,
        crate::api::calendar::generate_calendar_token,

        // ───── T-1739 pass 2 ─────
        // Accessibility
        crate::api::accessible::list_accessible_slots,
        crate::api::accessible::admin_set_slot_accessible,
        crate::api::accessible::accessible_stats,
        crate::api::accessible::update_accessibility_needs,

        // Admin — bulk user ops, reports, detailed health, booking policies
        crate::api::admin_ext::bulk_update_users,
        crate::api::admin_ext::bulk_delete_users,
        crate::api::admin_ext::revenue_report,
        crate::api::admin_ext::occupancy_report,
        crate::api::admin_ext::user_report,
        crate::api::admin_ext::detailed_health_check,
        crate::api::admin_ext::get_booking_policies,
        crate::api::admin_ext::update_booking_policies,
        crate::api::admin_ext::get_notification_preferences,
        crate::api::admin_ext::update_notification_preferences,

        // Admin — password reset
        crate::api::admin_handlers::admin_reset_user_password,

        // API Docs / Swagger UI / Postman collection
        crate::api::api_docs::api_docs_ui,
        crate::api::api_docs::api_docs_openapi_json,
        crate::api::api_docs::api_docs_postman_json,

        // Audit log export — enhanced + signed download
        crate::api::audit_export::enhanced_audit_export,
        crate::api::audit_export::download_audit_export,

        // Calendar drag (reschedule booking)
        crate::api::calendar_drag::reschedule_booking,

        // Credits — transactions ledger
        crate::api::credits::admin_list_credit_transactions,

        // Data management — import + bulk CSV export
        crate::api::data_management::import_users,
        crate::api::data_management::import_lots,
        crate::api::data_management::export_lots_csv,
        crate::api::data_management::export_bookings_csv,
        crate::api::data_management::export_users_csv,

        // Dynamic pricing
        crate::api::dynamic_pricing::get_dynamic_pricing,
        crate::api::dynamic_pricing::admin_get_dynamic_pricing_rules,
        crate::api::dynamic_pricing::admin_update_dynamic_pricing_rules,

        // EV charging
        crate::api::ev_charging::list_lot_chargers,
        crate::api::ev_charging::start_charging,
        crate::api::ev_charging::stop_charging,
        crate::api::ev_charging::charging_history,
        crate::api::ev_charging::admin_charger_overview,
        crate::api::ev_charging::admin_add_charger,

        // Fleet management (admin)
        crate::api::fleet::admin_fleet_list,
        crate::api::fleet::admin_fleet_stats,
        crate::api::fleet::admin_fleet_flag,

        // Guest bookings (user-facing list)
        crate::api::guest::list_user_guest_bookings,

        // Import — absences iCal
        crate::api::import::import_absences_ical,

        // Invoices — PDF
        crate::api::invoices::get_booking_invoice_pdf,

        // Lobby (public display)
        crate::api::lobby::lot_display,

        // Map (lot markers + admin location)
        crate::api::map::list_lot_markers,
        crate::api::map::set_lot_location,

        // Mobile quick endpoints
        crate::api::mobile::nearby_lots,
        crate::api::mobile::quick_book,
        crate::api::mobile::active_booking,

        // Notification center (in-app notifications)
        crate::api::notification_center::list_center_notifications,
        crate::api::notification_center::unread_count,
        crate::api::notification_center::delete_notification,
        crate::api::notification_center::mark_all_read,

        // Operating hours
        crate::api::operating_hours::get_operating_hours,
        crate::api::operating_hours::admin_update_operating_hours,

        // Parking pass (digital passes + verify)
        crate::api::parking_pass::get_booking_pass,
        crate::api::parking_pass::verify_pass,
        crate::api::parking_pass::list_my_passes,

        // Parking zones pricing
        crate::api::parking_zones::list_zones_pricing,
        crate::api::parking_zones::set_zone_pricing,
        crate::api::parking_zones::get_zone_price,

        // QR — slot QR code
        crate::api::qr::slot_qr_code,

        // Rate limits dashboard (admin)
        crate::api::rate_dashboard::admin_rate_limit_stats,
        crate::api::rate_dashboard::admin_rate_limit_history,

        // Recommendations — stats
        crate::api::recommendations::get_recommendation_stats,

        // Recurring bookings — update
        crate::api::recurring::update_recurring_booking,

        // Stripe (real checkout, webhook, history, config)
        crate::api::stripe::create_checkout,
        crate::api::stripe::stripe_webhook,
        crate::api::stripe::payment_history,
        crate::api::stripe::stripe_config,

        // Waitlist extended (subscribe, list, leave, accept, decline)
        crate::api::waitlist_ext::subscribe_waitlist,
        crate::api::waitlist_ext::get_lot_waitlist,
        crate::api::waitlist_ext::leave_lot_waitlist,
        crate::api::waitlist_ext::accept_waitlist_offer,
        crate::api::waitlist_ext::decline_waitlist_offer,

        // Admin widgets (dashboard layout + data)
        crate::api::widgets::get_widget_layout,
        crate::api::widgets::save_widget_layout,
        crate::api::widgets::get_widget_data,
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
    fn test_openapi_has_public_contract_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in [
            "/api/v1/health",
            "/api/v1/health/live",
            "/api/v1/health/ready",
            "/api/v1/health/info",
            "/api/v1/discover",
        ] {
            assert!(json.contains(path), "Missing path: {path}");
        }
        assert!(json.contains("realtime_transport"));
    }

    #[test]
    fn test_openapi_keeps_public_module_response_schemas() {
        let doc = ApiDoc::openapi();
        let value: serde_json::Value = serde_json::from_str(&doc.to_json().unwrap()).unwrap();

        let modules_get = &value["paths"]["/api/v1/modules"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"];
        assert_eq!(
            modules_get.as_str(),
            Some("#/components/schemas/ListModulesResponse")
        );

        let module_get_ok = &value["paths"]["/api/v1/modules/{name}"]["get"]["responses"]["200"]["content"]
            ["application/json"]["schema"]["$ref"];
        assert_eq!(
            module_get_ok.as_str(),
            Some("#/components/schemas/ModuleInfoResponseSchema")
        );

        let module_get_not_found = &value["paths"]["/api/v1/modules/{name}"]["get"]["responses"]["404"]
            ["content"]["application/json"]["schema"]["$ref"];
        assert_eq!(
            module_get_not_found.as_str(),
            Some("#/components/schemas/ModuleInfoResponseSchema")
        );

        let module_patch_ok = &value["paths"]["/api/v1/admin/modules/{name}"]["patch"]["responses"]
            ["200"]["content"]["application/json"]["schema"]["$ref"];
        assert_eq!(
            module_patch_ok.as_str(),
            Some("#/components/schemas/ModuleInfoResponseSchema")
        );

        let module_config_patch_ok = &value["paths"]["/api/v1/admin/modules/{name}/config"]["patch"]
            ["responses"]["200"]["content"]["application/json"]["schema"]["$ref"];
        assert_eq!(
            module_config_patch_ok.as_str(),
            Some("#/components/schemas/ModuleConfigResponseSchema")
        );

        let module_config_patch_validation = &value["paths"]["/api/v1/admin/modules/{name}/config"]
            ["patch"]["responses"]["422"]["content"]["application/json"]["schema"]["$ref"];
        assert_eq!(
            module_config_patch_validation.as_str(),
            Some("#/components/schemas/ModuleConfigResponseSchema")
        );
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
