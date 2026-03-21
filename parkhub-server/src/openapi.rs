//! OpenAPI Documentation
//!
//! Generates OpenAPI 3.0 specification and Swagger UI.

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
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
        description = "Open source parking lot management system API",
        license(name = "MIT", url = "https://opensource.org/licenses/MIT"),
        contact(
            name = "ParkHub",
            url = "https://github.com/nash87/parkhub"
        )
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    tags(
        (name = "Authentication", description = "User authentication endpoints"),
        (name = "Users", description = "User management"),
        (name = "Bookings", description = "Parking bookings"),
        (name = "Lots", description = "Parking lots and slots"),
        (name = "Vehicles", description = "User vehicles"),
        (name = "Health", description = "Health check endpoints"),
        (name = "Monitoring", description = "Metrics and monitoring"),
        (name = "Admin", description = "Administrative endpoints"),
        (name = "Credits", description = "Credit balance and management")
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

        // Credits
        crate::api::credits::get_user_credits,
        crate::api::credits::admin_grant_credits,
        crate::api::credits::admin_refill_all_credits,
        crate::api::credits::admin_update_user_quota,

        // QR Pass
        crate::api::qr::booking_qr_code,
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
        for tag in ["Authentication", "Lots", "Credits", "Health", "Monitoring"] {
            assert!(json.contains(tag), "Missing tag: {tag}");
        }
    }

    #[test]
    fn test_openapi_has_auth_paths() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for path in ["/auth/login", "/auth/register", "/auth/refresh"] {
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
    fn test_openapi_has_schemas() {
        let doc = ApiDoc::openapi();
        let json = doc.to_json().unwrap();
        for schema in ["LoginRequest", "RegisterRequest", "CreateParkingLotRequest"] {
            assert!(json.contains(schema), "Missing schema: {schema}");
        }
    }
}
