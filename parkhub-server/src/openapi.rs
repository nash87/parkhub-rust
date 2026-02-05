//! OpenAPI Documentation
//!
//! Generates OpenAPI 3.0 specification and Swagger UI.

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use axum::Router;

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
        (name = "Admin", description = "Administrative endpoints")
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
        // Health endpoints will be added via #[utoipa::path] macros
    )
)]
pub struct ApiDoc;

/// Create Swagger UI router
pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
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
}
