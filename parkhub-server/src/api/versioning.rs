//! API Versioning & Deprecation.
//!
//! Provides version information, deprecation notices, and changelog
//! for the ParkHub REST API.
//!
//! - `GET /api/v1/version`    — current API version + deprecation notices
//! - `GET /api/v1/changelog`  — API changelog (breaking changes, new endpoints)
//!
//! Additionally, an Axum middleware layer injects:
//! - `X-API-Version: <version>` on all responses
//! - `Sunset: <date>` on deprecated endpoints

use axum::{
    body::Body,
    extract::State,
    http::{header::HeaderName, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};

use parkhub_common::ApiResponse;

use super::SharedState;

/// The current API version, derived from Cargo.toml at compile time.
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Custom header name for API version
static X_API_VERSION: HeaderName = HeaderName::from_static("x-api-version");

/// Custom header name for deprecation sunset date
static SUNSET: HeaderName = HeaderName::from_static("sunset");

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Deprecation severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeprecationSeverity {
    /// Informational notice — endpoint still works
    Info,
    /// Warning — endpoint will be removed in a future version
    Warning,
    /// Critical — endpoint is scheduled for imminent removal
    Critical,
}

impl DeprecationSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
        }
    }
}

/// A deprecation notice for a specific endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeprecationNotice {
    pub endpoint: String,
    pub method: String,
    pub severity: DeprecationSeverity,
    pub message: String,
    pub sunset_date: Option<String>,
    pub replacement: Option<String>,
}

/// API version info response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiVersionInfo {
    pub version: String,
    pub api_prefix: String,
    pub status: String,
    pub deprecations: Vec<DeprecationNotice>,
    pub supported_versions: Vec<String>,
}

/// A single changelog entry
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub changes: Vec<ChangeItem>,
}

/// Type of change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Breaking,
    Feature,
    Fix,
    Deprecation,
}

impl ChangeType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Breaking => "Breaking",
            Self::Feature => "Feature",
            Self::Fix => "Fix",
            Self::Deprecation => "Deprecation",
        }
    }
}

/// A single change item within a version
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChangeItem {
    pub change_type: ChangeType,
    pub description: String,
    pub endpoint: Option<String>,
}

/// Changelog response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiChangelog {
    pub current_version: String,
    pub entries: Vec<ChangelogEntry>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// DATA GENERATORS
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate current deprecation notices
fn generate_deprecation_notices() -> Vec<DeprecationNotice> {
    vec![DeprecationNotice {
        endpoint: "/api/v1/lots/{id}/slots".to_string(),
        method: "GET".to_string(),
        severity: DeprecationSeverity::Info,
        message: "Use /api/v1/lots/{id}/display for enhanced lot information".to_string(),
        sunset_date: Some("2027-01-01".to_string()),
        replacement: Some("/api/v1/lots/{id}/display".to_string()),
    }]
}

/// Generate the API changelog
fn generate_changelog() -> Vec<ChangelogEntry> {
    vec![
        ChangelogEntry {
            version: "4.1.0".to_string(),
            date: "2026-03-23".to_string(),
            changes: vec![
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "Booking sharing & guest invites (mod-sharing)".to_string(),
                    endpoint: Some("/api/v1/bookings/{id}/share".to_string()),
                },
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "Scheduled email reports (mod-scheduled-reports)".to_string(),
                    endpoint: Some("/api/v1/admin/reports/schedules".to_string()),
                },
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "API versioning & deprecation headers".to_string(),
                    endpoint: Some("/api/v1/version".to_string()),
                },
            ],
        },
        ChangelogEntry {
            version: "4.0.0".to_string(),
            date: "2026-03-20".to_string(),
            changes: vec![
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "Plugin system with event hooks".to_string(),
                    endpoint: Some("/api/v1/admin/plugins".to_string()),
                },
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "GraphQL API with playground".to_string(),
                    endpoint: Some("/api/v1/graphql".to_string()),
                },
                ChangeItem {
                    change_type: ChangeType::Feature,
                    description: "GDPR/DSGVO compliance reports".to_string(),
                    endpoint: Some("/api/v1/admin/compliance/report".to_string()),
                },
            ],
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

/// Middleware that injects `X-API-Version` and optional `Sunset` headers on all responses.
pub async fn api_version_middleware(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    // Always inject the API version header
    if let Ok(val) = HeaderValue::from_str(API_VERSION) {
        response.headers_mut().insert(X_API_VERSION.clone(), val);
    }

    // Check if this endpoint has a sunset date
    let deprecations = generate_deprecation_notices();
    for notice in &deprecations {
        if path.contains(&notice.endpoint.replace("{id}", "")) || path == notice.endpoint {
            if let Some(ref sunset) = notice.sunset_date {
                if let Ok(val) = HeaderValue::from_str(sunset) {
                    response.headers_mut().insert(SUNSET.clone(), val);
                }
            }
        }
    }

    response
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/version` — current API version and deprecation notices.
pub async fn api_version(
    State(_state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<ApiVersionInfo>>) {
    let info = ApiVersionInfo {
        version: API_VERSION.to_string(),
        api_prefix: "/api/v1".to_string(),
        status: "stable".to_string(),
        deprecations: generate_deprecation_notices(),
        supported_versions: vec!["v1".to_string()],
    };

    (StatusCode::OK, Json(ApiResponse::success(info)))
}

/// `GET /api/v1/changelog` — API changelog with breaking changes and new endpoints.
pub async fn api_changelog(
    State(_state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<ApiChangelog>>) {
    let changelog = ApiChangelog {
        current_version: API_VERSION.to_string(),
        entries: generate_changelog(),
    };

    (StatusCode::OK, Json(ApiResponse::success(changelog)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_version_constant() {
        assert!(!API_VERSION.is_empty());
        // Should be a semver-like version
        assert!(API_VERSION.contains('.'));
    }

    #[test]
    fn test_deprecation_severity_labels() {
        assert_eq!(DeprecationSeverity::Info.label(), "Info");
        assert_eq!(DeprecationSeverity::Warning.label(), "Warning");
        assert_eq!(DeprecationSeverity::Critical.label(), "Critical");
    }

    #[test]
    fn test_deprecation_severity_serialize() {
        assert_eq!(
            serde_json::to_string(&DeprecationSeverity::Info).unwrap(),
            "\"info\""
        );
        assert_eq!(
            serde_json::to_string(&DeprecationSeverity::Warning).unwrap(),
            "\"warning\""
        );
        assert_eq!(
            serde_json::to_string(&DeprecationSeverity::Critical).unwrap(),
            "\"critical\""
        );
    }

    #[test]
    fn test_change_type_labels() {
        assert_eq!(ChangeType::Breaking.label(), "Breaking");
        assert_eq!(ChangeType::Feature.label(), "Feature");
        assert_eq!(ChangeType::Fix.label(), "Fix");
        assert_eq!(ChangeType::Deprecation.label(), "Deprecation");
    }

    #[test]
    fn test_change_type_serialize() {
        assert_eq!(
            serde_json::to_string(&ChangeType::Breaking).unwrap(),
            "\"breaking\""
        );
        assert_eq!(
            serde_json::to_string(&ChangeType::Feature).unwrap(),
            "\"feature\""
        );
    }

    #[test]
    fn test_generate_deprecation_notices() {
        let notices = generate_deprecation_notices();
        assert!(!notices.is_empty());
        assert!(notices[0].endpoint.contains("lots"));
        assert!(notices[0].sunset_date.is_some());
    }

    #[test]
    fn test_generate_changelog() {
        let changelog = generate_changelog();
        assert!(changelog.len() >= 2);
        assert_eq!(changelog[0].version, "4.1.0");
        assert_eq!(changelog[1].version, "4.0.0");
        assert!(!changelog[0].changes.is_empty());
    }

    #[test]
    fn test_api_version_info_serialize() {
        let info = ApiVersionInfo {
            version: "4.1.0".to_string(),
            api_prefix: "/api/v1".to_string(),
            status: "stable".to_string(),
            deprecations: vec![],
            supported_versions: vec!["v1".to_string()],
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"version\":\"4.1.0\""));
        assert!(json.contains("\"status\":\"stable\""));
        assert!(json.contains("\"api_prefix\":\"/api/v1\""));
    }

    #[test]
    fn test_changelog_entry_serialize() {
        let entry = ChangelogEntry {
            version: "4.1.0".to_string(),
            date: "2026-03-23".to_string(),
            changes: vec![ChangeItem {
                change_type: ChangeType::Feature,
                description: "Test feature".to_string(),
                endpoint: Some("/api/v1/test".to_string()),
            }],
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"change_type\":\"feature\""));
        assert!(json.contains("\"description\":\"Test feature\""));
    }

    #[test]
    fn test_deprecation_notice_serialize() {
        let notice = DeprecationNotice {
            endpoint: "/api/v1/old".to_string(),
            method: "GET".to_string(),
            severity: DeprecationSeverity::Warning,
            message: "Use /api/v1/new instead".to_string(),
            sunset_date: Some("2027-01-01".to_string()),
            replacement: Some("/api/v1/new".to_string()),
        };
        let json = serde_json::to_string(&notice).unwrap();
        assert!(json.contains("\"severity\":\"warning\""));
        assert!(json.contains("\"sunset_date\":\"2027-01-01\""));
        assert!(json.contains("\"replacement\":\"/api/v1/new\""));
    }
}
