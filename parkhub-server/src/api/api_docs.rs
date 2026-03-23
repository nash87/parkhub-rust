//! Interactive API Documentation handlers.
//!
//! Provides convenient endpoints for accessing the API documentation.
//!
//! - `GET /api/v1/docs` — serve embedded Swagger UI HTML page
//! - `GET /api/v1/docs/openapi.json` — raw OpenAPI 3.0 specification
//! - `GET /api/v1/docs/postman.json` — auto-generated Postman collection

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

/// `GET /api/v1/docs/postman.json` — auto-generated Postman collection from OpenAPI spec
#[utoipa::path(get, path = "/api/v1/docs/postman.json", tag = "Documentation",
    summary = "Postman collection",
    description = "Returns an auto-generated Postman v2.1 collection derived from the OpenAPI specification. Import directly into Postman.",
    responses(
        (status = 200, description = "Postman collection JSON"),
    )
)]
pub async fn api_docs_postman_json() -> impl IntoResponse {
    let collection = generate_postman_collection();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        collection,
    )
}

/// Convert OpenAPI spec into a Postman v2.1 collection.
fn generate_postman_collection() -> String {
    let spec_str = generate_openapi_spec();
    let spec: serde_json::Value = serde_json::from_str(&spec_str).unwrap_or_default();

    let mut folders: std::collections::BTreeMap<String, Vec<serde_json::Value>> =
        std::collections::BTreeMap::new();

    if let Some(paths) = spec["paths"].as_object() {
        for (path, methods) in paths {
            if let Some(methods_obj) = methods.as_object() {
                for (method, details) in methods_obj {
                    let tag = details["tags"]
                        .as_array()
                        .and_then(|t| t.first())
                        .and_then(|v| v.as_str())
                        .unwrap_or("General")
                        .to_string();

                    let summary = details["summary"]
                        .as_str()
                        .unwrap_or(path.as_str())
                        .to_string();

                    let url_parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
                    let host = vec!["{{base_url}}".to_string()];

                    // Convert {param} to :param for Postman
                    let path_segments: Vec<String> = url_parts
                        .iter()
                        .map(|s| {
                            if s.starts_with('{') && s.ends_with('}') {
                                format!(":{}", &s[1..s.len() - 1])
                            } else {
                                s.to_string()
                            }
                        })
                        .collect();

                    let mut request = serde_json::json!({
                        "method": method.to_uppercase(),
                        "header": [
                            { "key": "Content-Type", "value": "application/json" },
                            { "key": "Accept", "value": "application/json" }
                        ],
                        "url": {
                            "raw": format!("{{{{base_url}}}}{path}"),
                            "host": host,
                            "path": path_segments,
                        }
                    });

                    // Add auth header for non-public endpoints
                    let is_public = tag == "Health"
                        || tag == "Documentation"
                        || path.contains("/public/")
                        || path.contains("/pass/verify");

                    if !is_public {
                        request["auth"] = serde_json::json!({
                            "type": "bearer",
                            "bearer": [{ "key": "token", "value": "{{token}}", "type": "string" }]
                        });
                    }

                    // Add example body for POST/PUT methods
                    if method == "post" || method == "put" {
                        if let Some(rb) = details.get("requestBody") {
                            if let Some(content) = rb.get("content") {
                                if let Some(json_content) = content.get("application/json") {
                                    if let Some(example) = json_content.get("example") {
                                        request["body"] = serde_json::json!({
                                            "mode": "raw",
                                            "raw": serde_json::to_string_pretty(example).unwrap_or_default()
                                        });
                                    }
                                }
                            }
                        }
                    }

                    let item = serde_json::json!({
                        "name": summary,
                        "request": request,
                    });

                    folders.entry(tag).or_default().push(item);
                }
            }
        }
    }

    let folder_items: Vec<serde_json::Value> = folders
        .into_iter()
        .map(|(name, items)| {
            serde_json::json!({
                "name": name,
                "item": items,
            })
        })
        .collect();

    serde_json::json!({
        "info": {
            "name": "ParkHub API (auto-generated)",
            "description": format!(
                "Auto-generated Postman collection from ParkHub v{} OpenAPI spec.\n\nSet the `base_url` and `token` environment variables.",
                env!("CARGO_PKG_VERSION")
            ),
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "auth": {
            "type": "bearer",
            "bearer": [{ "key": "token", "value": "{{token}}", "type": "string" }]
        },
        "variable": [
            { "key": "base_url", "value": "http://localhost:8080" },
            { "key": "token", "value": "" }
        ],
        "item": folder_items
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
        assert!(parsed["info"]["title"]
            .as_str()
            .unwrap()
            .contains("ParkHub"));
    }

    #[test]
    fn test_postman_collection_is_valid_json() {
        let collection = generate_postman_collection();
        let parsed: serde_json::Value = serde_json::from_str(&collection).unwrap();
        assert!(parsed["info"]["name"].as_str().unwrap().contains("ParkHub"));
        assert_eq!(
            parsed["info"]["schema"],
            "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        );
    }

    #[test]
    fn test_postman_collection_has_auth() {
        let collection = generate_postman_collection();
        let parsed: serde_json::Value = serde_json::from_str(&collection).unwrap();
        assert_eq!(parsed["auth"]["type"], "bearer");
        assert!(parsed["variable"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_postman_collection_has_folders() {
        let collection = generate_postman_collection();
        let parsed: serde_json::Value = serde_json::from_str(&collection).unwrap();
        let items = parsed["item"].as_array().unwrap();
        // Should have at least the Documentation folder from OpenAPI paths
        assert!(!items.is_empty(), "Collection should have folder items");
        // Each folder should have a name
        for item in items {
            assert!(
                item["name"].as_str().is_some(),
                "Each folder should have a name"
            );
        }
    }

    #[test]
    fn test_postman_collection_variables() {
        let collection = generate_postman_collection();
        let parsed: serde_json::Value = serde_json::from_str(&collection).unwrap();
        let vars = parsed["variable"].as_array().unwrap();
        let keys: Vec<&str> = vars.iter().filter_map(|v| v["key"].as_str()).collect();
        assert!(keys.contains(&"base_url"), "Should have base_url variable");
        assert!(keys.contains(&"token"), "Should have token variable");
    }
}
