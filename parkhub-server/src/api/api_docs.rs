//! Interactive API Documentation handlers.
//!
//! Provides convenient endpoints for accessing the API documentation.
//!
//! - `GET /api/v1/docs` — serve embedded Swagger UI HTML page
//! - `GET /api/v1/docs/openapi.json` — raw OpenAPI 3.0 specification

use axum::{
    http::{header, StatusCode},
    response::{Html, IntoResponse},
};

/// `GET /api/v1/docs` — serve Swagger UI as an embedded HTML page
#[utoipa::path(get, path = "/api/v1/docs", tag = "Documentation",
    summary = "Interactive API documentation",
    description = "Serves an embedded Swagger UI page for exploring and testing the ParkHub REST API.",
    responses(
        (status = 200, description = "Swagger UI HTML page"),
    )
)]
pub async fn api_docs_ui() -> impl IntoResponse {
    Html(SWAGGER_HTML)
}

/// `GET /api/v1/docs/openapi.json` — raw OpenAPI spec
#[utoipa::path(get, path = "/api/v1/docs/openapi.json", tag = "Documentation",
    summary = "OpenAPI specification",
    description = "Returns the raw OpenAPI 3.0 JSON specification for the ParkHub API.",
    responses(
        (status = 200, description = "OpenAPI JSON spec"),
    )
)]
pub async fn api_docs_openapi_json() -> impl IntoResponse {
    // Generate the OpenAPI spec at runtime
    let spec = generate_openapi_spec();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        spec,
    )
}

/// Generate the OpenAPI spec JSON string
fn generate_openapi_spec() -> String {
    serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "ParkHub API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Open-source, self-hosted parking lot management system. Provides endpoints for user authentication, booking management, parking lot administration, credit system, webhooks, digital passes, waitlist, and more.",
            "license": { "name": "MIT", "url": "https://opensource.org/licenses/MIT" },
            "contact": { "name": "ParkHub", "url": "https://github.com/nash87/parkhub-rust" }
        },
        "servers": [{ "url": "/", "description": "Local server" }],
        "tags": [
            { "name": "Authentication", "description": "User login, registration, password reset" },
            { "name": "Users", "description": "User profile and account management" },
            { "name": "Bookings", "description": "Create, list, cancel, and check in to parking bookings" },
            { "name": "Lots", "description": "Parking lots and slot management" },
            { "name": "Vehicles", "description": "User vehicle registration" },
            { "name": "Credits", "description": "Credit balance, grants, and quota" },
            { "name": "Favorites", "description": "User favorite parking slots" },
            { "name": "Waitlist", "description": "Waitlist management for full lots" },
            { "name": "Waitlist Extended", "description": "Enhanced waitlist with notifications, accept/decline offers" },
            { "name": "Parking Pass", "description": "Digital parking passes with QR verification" },
            { "name": "Notifications", "description": "In-app notification management" },
            { "name": "Calendar", "description": "Calendar events and iCal export" },
            { "name": "EV Charging", "description": "Electric vehicle charging station management" },
            { "name": "Geofence", "description": "Geofencing and auto check-in" },
            { "name": "History", "description": "Personal parking history and statistics" },
            { "name": "Admin", "description": "Administrative endpoints" },
            { "name": "Webhooks", "description": "Webhook configuration and event delivery" },
            { "name": "Health", "description": "Health check and readiness probes" },
            { "name": "Documentation", "description": "API documentation endpoints" }
        ],
        "paths": {
            "/api/v1/docs": {
                "get": {
                    "tags": ["Documentation"],
                    "summary": "Interactive API docs (Swagger UI)",
                    "responses": { "200": { "description": "HTML page" } }
                }
            },
            "/api/v1/docs/openapi.json": {
                "get": {
                    "tags": ["Documentation"],
                    "summary": "Raw OpenAPI specification",
                    "responses": { "200": { "description": "OpenAPI JSON" } }
                }
            }
        }
    })
    .to_string()
}

/// Embedded Swagger UI HTML template
const SWAGGER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ParkHub API Documentation</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
  <style>
    body { margin: 0; padding: 0; }
    .topbar { display: none !important; }
    .swagger-ui .info { margin: 20px 0; }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    SwaggerUIBundle({
      url: '/api/v1/docs/openapi.json',
      dom_id: '#swagger-ui',
      deepLinking: true,
      presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
      layout: 'BaseLayout',
      defaultModelsExpandDepth: 1,
      docExpansion: 'list',
      filter: true,
      tryItOutEnabled: true,
    });
  </script>
</body>
</html>"#;

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = generate_openapi_spec();
        assert!(spec.contains("ParkHub API"));
        assert!(spec.contains("openapi"));
        assert!(spec.contains("3.0.3"));
    }

    #[test]
    fn test_openapi_spec_has_tags() {
        let spec = generate_openapi_spec();
        for tag in [
            "Authentication",
            "Bookings",
            "Parking Pass",
            "Waitlist Extended",
            "Documentation",
        ] {
            assert!(spec.contains(tag), "Missing tag: {tag}");
        }
    }

    #[test]
    fn test_openapi_spec_has_paths() {
        let spec = generate_openapi_spec();
        assert!(spec.contains("/api/v1/docs"));
        assert!(spec.contains("/api/v1/docs/openapi.json"));
    }

    #[test]
    fn test_swagger_html_contains_elements() {
        assert!(SWAGGER_HTML.contains("swagger-ui"));
        assert!(SWAGGER_HTML.contains("SwaggerUIBundle"));
        assert!(SWAGGER_HTML.contains("/api/v1/docs/openapi.json"));
        assert!(SWAGGER_HTML.contains("ParkHub API Documentation"));
    }

    #[test]
    fn test_openapi_spec_is_valid_json() {
        let spec = generate_openapi_spec();
        let parsed: serde_json::Value = serde_json::from_str(&spec).unwrap();
        assert_eq!(parsed["openapi"], "3.0.3");
        assert!(parsed["info"]["title"].as_str().unwrap().contains("ParkHub"));
    }
}
