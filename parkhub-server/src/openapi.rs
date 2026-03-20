//! OpenAPI Documentation
//!
//! Generates OpenAPI 3.0 specification and Swagger UI.

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
    requests::*,
};

/// OpenAPI documentation
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
        (name = "Public", description = "Unauthenticated public endpoints")
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
            AdminUserResponse,
            UpdateQuotaRequest,

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
    )
)]
pub struct ApiDoc;

/// Create Swagger UI router
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}

/// Add OpenAPI routes to router
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
            "Lots",
            "Zones",
            "Credits",
            "Webhooks",
            "Favorites",
            "Push",
            "Setup",
            "Admin",
            "Health",
            "Monitoring",
            "Public",
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
        for path in ["/admin/users/export-csv", "/admin/bookings/export-csv"] {
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
}
