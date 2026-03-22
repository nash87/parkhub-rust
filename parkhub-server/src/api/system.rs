//! System handlers: health checks, version, maintenance, handshake, server status, metrics.
//!
//! These endpoints provide operational observability and protocol negotiation.
//! None require authentication (they are public/infrastructure endpoints).

use axum::{
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::Instrument;

use parkhub_common::{
    ApiResponse, HandshakeRequest, HandshakeResponse, ServerStatus, PROTOCOL_VERSION,
};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH & DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════════

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    summary = "Basic health check",
    description = "Returns 200 OK when the server is running.",
    responses((status = 200, description = "Healthy"))
)]
#[tracing::instrument]
pub async fn health_check() -> &'static str {
    "OK"
}

/// Kubernetes liveness probe - just checks if the service is running
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "Health",
    summary = "Kubernetes liveness probe",
    description = "Returns 200 if the process is alive.",
    responses((status = 200, description = "Alive"))
)]
#[tracing::instrument]
pub async fn liveness_check() -> StatusCode {
    StatusCode::OK
}

/// Kubernetes readiness probe - checks if the service can handle traffic.
///
/// Returns only a boolean `ready` field. Internal error details are logged
/// server-side but never exposed in the response body.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "Health",
    summary = "Kubernetes readiness probe",
    description = "Returns 200 when the service can accept traffic.",
    responses(
        (status = 200, description = "Ready"),
        (status = 503, description = "Not ready")
    )
)]
#[tracing::instrument(skip(state))]
pub async fn readiness_check(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    match state.db.stats().await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"ready": true}))),
        Err(e) => {
            tracing::error!(error = %e, "Readiness check failed — database unavailable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"ready": false})),
            )
        }
    }
}

/// `GET /api/v1/system/version` — server version information
pub async fn system_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
    }))
}

/// `GET /api/v1/system/maintenance` — maintenance mode status
pub async fn system_maintenance(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let maintenance = match state.db.get_setting("maintenance_mode").await {
        Ok(Some(v)) => v == "true",
        _ => false,
    };
    Json(serde_json::json!({
        "maintenance_mode": maintenance,
        "message": if maintenance { "System is under maintenance" } else { "" }
    }))
}

#[utoipa::path(
    post,
    path = "/handshake",
    tag = "Health",
    summary = "Protocol handshake",
    description = "Verifies protocol version compatibility between client and server.",
    responses((status = 200, description = "Handshake result"))
)]
pub async fn handshake(
    State(state): State<SharedState>,
    Json(request): Json<HandshakeRequest>,
) -> Json<ApiResponse<HandshakeResponse>> {
    let state = state.read().await;

    // Check protocol version compatibility
    if request.protocol_version != PROTOCOL_VERSION {
        return Json(ApiResponse::error(
            "PROTOCOL_MISMATCH",
            format!(
                "Protocol version mismatch: server={}, client={}",
                PROTOCOL_VERSION, request.protocol_version
            ),
        ));
    }

    Json(ApiResponse::success(HandshakeResponse {
        server_name: state.config.server_name.clone(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: PROTOCOL_VERSION.to_string(),
        requires_auth: true,
        certificate_fingerprint: String::new(),
    }))
}

#[utoipa::path(
    get,
    path = "/status",
    tag = "Health",
    summary = "Server status overview",
    description = "Returns aggregate server statistics.",
    responses((status = 200, description = "Server status"))
)]
pub async fn server_status(State(state): State<SharedState>) -> Json<ApiResponse<ServerStatus>> {
    let db_stats = {
        let state = state.read().await;
        state.db.stats().await.unwrap_or(crate::db::DatabaseStats {
            users: 0,
            bookings: 0,
            parking_lots: 0,
            slots: 0,
            sessions: 0,
            vehicles: 0,
        })
    };

    Json(ApiResponse::success(ServerStatus {
        uptime_seconds: 0,
        connected_clients: 0,
        total_users: u32::try_from(db_stats.users).unwrap_or(u32::MAX),
        total_bookings: u32::try_from(db_stats.bookings).unwrap_or(u32::MAX),
        database_size_bytes: 0,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════════
// MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

/// Middleware that adds security-related response headers to every request.
///
/// - `X-Content-Type-Options`: prevents MIME sniffing
/// - `X-Frame-Options`: prevents clickjacking
/// - `Content-Security-Policy`: restricts resource origins
/// - `Referrer-Policy`: limits referrer leakage
/// - `Permissions-Policy`: disables unneeded browser features
/// - `Strict-Transport-Security`: enforces HTTPS for 1 year including subdomains
pub async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Safety: all header names and values below are valid ASCII strings.
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self' ws: wss:; \
             frame-ancestors 'none'",
        ),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), camera=(), microphone=()"),
    );
    headers.insert(
        HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    response
}

// ═══════════════════════════════════════════════════════════════════════════════
// REQUEST ID TRACING MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

/// Attaches the `x-request-id` header value (set by `SetRequestIdLayer`) to the
/// current tracing span so that every log line within this request includes the
/// request ID.  Also ensures the header is copied to the response.
pub async fn request_id_tracing_middleware(request: Request<Body>, next: Next) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    let span = tracing::info_span!("request", request_id = %request_id);
    let guard = span.enter();

    let mut response = {
        // Drop the span guard before awaiting so the span covers the full
        // request lifecycle via the instrument approach.
        drop(guard);
        next.run(request).instrument(span).await
    };

    // Ensure x-request-id is on the response (belt-and-suspenders with PropagateRequestIdLayer)
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .entry(HeaderName::from_static("x-request-id"))
            .or_insert(val);
    }

    response
}

// ═══════════════════════════════════════════════════════════════════════════════
// HTTP METRICS MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

/// Records HTTP request metrics (method, path, status, duration) for Prometheus
/// and emits a structured log line for every request.
pub async fn http_metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let status = response.status().as_u16();
    let duration = start.elapsed();

    // Normalize path to avoid high-cardinality labels (strip UUIDs/IDs)
    let normalized = normalize_metric_path(&path);
    crate::metrics::record_http_request(&method, &normalized, status, duration);

    // Structured request log — every request gets one line with key fields
    tracing::info!(
        http.method = %method,
        http.path = %path,
        http.status = status,
        http.latency_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        "request completed"
    );

    response
}

/// Collapse dynamic path segments (UUIDs, numeric IDs) into placeholders
/// to keep Prometheus label cardinality bounded.
pub fn normalize_metric_path(path: &str) -> String {
    path.split('/')
        .map(|s| {
            let is_uuid = s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4;
            let is_numeric = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            if is_uuid || is_numeric {
                ":id"
            } else {
                s
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

// ═══════════════════════════════════════════════════════════════════════════════
// SETUP WIZARD (mod-setup-wizard)
// ═══════════════════════════════════════════════════════════════════════════════

/// Step tracking for the setup wizard
#[cfg(feature = "mod-setup-wizard")]
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, utoipa::ToSchema)]
pub struct WizardStep {
    pub step: u8,
    pub name: String,
    pub completed: bool,
}

/// Status response for the setup wizard
#[cfg(feature = "mod-setup-wizard")]
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct WizardStatus {
    pub completed: bool,
    pub steps: Vec<WizardStep>,
}

/// Wizard step payload — each step has its own data
#[cfg(feature = "mod-setup-wizard")]
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct WizardStepRequest {
    /// Which step (1-4)
    pub step: u8,
    /// Step 1: Company info
    #[serde(default)]
    pub company_name: Option<String>,
    #[serde(default)]
    pub logo_base64: Option<String>,
    #[serde(default)]
    pub timezone: Option<String>,
    /// Step 2: Create lot
    #[serde(default)]
    pub lot_name: Option<String>,
    #[serde(default)]
    pub floor_count: Option<i32>,
    #[serde(default)]
    pub slots_per_floor: Option<i32>,
    /// Step 3: Invite users (comma-separated emails)
    #[serde(default)]
    pub invite_emails: Option<Vec<String>>,
    /// Step 4: Theme choice
    #[serde(default)]
    pub theme: Option<String>,
}

/// `GET /api/v1/setup/wizard/status` — check wizard progress
#[cfg(feature = "mod-setup-wizard")]
#[utoipa::path(
    get,
    path = "/api/v1/setup/wizard/status",
    tag = "Setup",
    summary = "Get wizard progress",
    description = "Returns whether the setup wizard is completed and per-step status.",
    responses(
        (status = 200, description = "Wizard status"),
    )
)]
pub async fn wizard_status(State(state): State<SharedState>) -> Json<ApiResponse<WizardStatus>> {
    let guard = state.read().await;

    let step1_done = guard
        .db
        .get_setting("wizard_step1_done")
        .await
        .unwrap_or(None)
        .unwrap_or_default()
        == "true";
    let step2_done = guard
        .db
        .get_setting("wizard_step2_done")
        .await
        .unwrap_or(None)
        .unwrap_or_default()
        == "true";
    let step3_done = guard
        .db
        .get_setting("wizard_step3_done")
        .await
        .unwrap_or(None)
        .unwrap_or_default()
        == "true";
    let step4_done = guard
        .db
        .get_setting("wizard_step4_done")
        .await
        .unwrap_or(None)
        .unwrap_or_default()
        == "true";

    let all_done = step1_done && step2_done && step3_done && step4_done;

    Json(ApiResponse::success(WizardStatus {
        completed: all_done,
        steps: vec![
            WizardStep {
                step: 1,
                name: "Company Info".to_string(),
                completed: step1_done,
            },
            WizardStep {
                step: 2,
                name: "Create Lot".to_string(),
                completed: step2_done,
            },
            WizardStep {
                step: 3,
                name: "User Setup".to_string(),
                completed: step3_done,
            },
            WizardStep {
                step: 4,
                name: "Choose Theme".to_string(),
                completed: step4_done,
            },
        ],
    }))
}

/// Available design themes for the wizard
#[cfg(feature = "mod-setup-wizard")]
const WIZARD_THEMES: &[&str] = &[
    "classic",
    "glass",
    "bento",
    "brutalist",
    "neon",
    "warm",
    "liquid",
    "mono",
    "ocean",
    "forest",
    "synthwave",
    "zen",
];

/// `POST /api/v1/setup/wizard` — process a single wizard step
#[cfg(feature = "mod-setup-wizard")]
#[utoipa::path(
    post,
    path = "/api/v1/setup/wizard",
    tag = "Setup",
    summary = "Process wizard step",
    description = "Processes a single step of the onboarding wizard. \
        Steps: 1 (company info), 2 (create lot), 3 (user invites), 4 (theme). \
        Only accessible when setup is not completed, or by admin.",
    responses(
        (status = 200, description = "Step processed"),
        (status = 400, description = "Invalid step or validation error"),
    )
)]
#[allow(clippy::too_many_lines)]
pub async fn wizard_step(
    State(state): State<SharedState>,
    Json(req): Json<WizardStepRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let guard = state.read().await;

    match req.step {
        1 => {
            // Step 1: Company info
            let name = match &req.company_name {
                Some(n) if !n.trim().is_empty() => n.trim(),
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::error(
                            "VALIDATION_ERROR",
                            "Company name is required",
                        )),
                    );
                }
            };

            let _ = guard.db.set_setting("company_name", name).await;

            if let Some(tz) = &req.timezone {
                let _ = guard.db.set_setting("timezone", tz).await;
            }

            if let Some(logo) = &req.logo_base64 {
                let _ = guard.db.set_setting("company_logo_base64", logo).await;
            }

            let _ = guard.db.set_setting("wizard_step1_done", "true").await;

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "step": 1,
                    "message": "Company info saved"
                }))),
            )
        }
        2 => {
            // Step 2: Create lot
            let lot_name = match &req.lot_name {
                Some(n) if !n.trim().is_empty() => n.trim().to_string(),
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::error(
                            "VALIDATION_ERROR",
                            "Lot name is required",
                        )),
                    );
                }
            };

            let floors = req.floor_count.unwrap_or(1).clamp(1, 20);
            let slots_per = req.slots_per_floor.unwrap_or(10).clamp(1, 500);
            let total = floors * slots_per;

            let lot = parkhub_common::ParkingLot {
                id: uuid::Uuid::new_v4(),
                name: lot_name.clone(),
                address: String::new(),
                latitude: 0.0,
                longitude: 0.0,
                total_slots: total,
                available_slots: total,
                floors: (1..=floors)
                    .map(|f| parkhub_common::ParkingFloor {
                        id: uuid::Uuid::new_v4(),
                        lot_id: uuid::Uuid::nil(),
                        name: format!("Floor {f}"),
                        floor_number: f,
                        total_slots: slots_per,
                        available_slots: slots_per,
                        slots: Vec::new(),
                    })
                    .collect(),
                amenities: Vec::new(),
                pricing: parkhub_common::PricingInfo {
                    currency: "EUR".to_string(),
                    rates: vec![parkhub_common::PricingRate {
                        duration_minutes: 60,
                        price: 2.0,
                        label: "1 hour".to_string(),
                    }],
                    daily_max: Some(15.0),
                    monthly_pass: None,
                },
                operating_hours: parkhub_common::OperatingHours {
                    is_24h: true,
                    monday: None,
                    tuesday: None,
                    wednesday: None,
                    thursday: None,
                    friday: None,
                    saturday: None,
                    sunday: None,
                },
                images: Vec::new(),
                status: parkhub_common::LotStatus::Open,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            if let Err(e) = guard.db.save_parking_lot(&lot).await {
                tracing::error!("Wizard: failed to create lot: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to create lot")),
                );
            }

            let _ = guard.db.set_setting("wizard_step2_done", "true").await;

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "step": 2,
                    "message": "Lot created",
                    "lot_id": lot.id.to_string(),
                    "total_slots": total
                }))),
            )
        }
        3 => {
            // Step 3: User invites (store emails for later invitation)
            if let Some(emails) = &req.invite_emails {
                let valid: Vec<_> = emails
                    .iter()
                    .filter(|e| e.contains('@') && e.len() >= 5)
                    .cloned()
                    .collect();
                let _ = guard
                    .db
                    .set_setting("wizard_invite_emails", &valid.join(","))
                    .await;
            }

            let _ = guard.db.set_setting("wizard_step3_done", "true").await;

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "step": 3,
                    "message": "User setup saved"
                }))),
            )
        }
        4 => {
            // Step 4: Theme
            let theme = req.theme.as_deref().unwrap_or("classic");
            if !WIZARD_THEMES.contains(&theme) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_THEME",
                        "Invalid theme selection",
                    )),
                );
            }

            let _ = guard.db.set_setting("default_design_theme", theme).await;
            let _ = guard.db.set_setting("wizard_step4_done", "true").await;

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "step": 4,
                    "message": "Theme saved",
                    "theme": theme
                }))),
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_STEP", "Step must be 1-4")),
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_metric_path_uuid() {
        let path = "/api/v1/lots/550e8400-e29b-41d4-a716-446655440000/slots";
        assert_eq!(normalize_metric_path(path), "/api/v1/lots/:id/slots");
    }

    #[test]
    fn test_normalize_metric_path_numeric() {
        let path = "/api/v1/bookings/12345";
        assert_eq!(normalize_metric_path(path), "/api/v1/bookings/:id");
    }

    #[test]
    fn test_normalize_metric_path_no_ids() {
        let path = "/api/v1/lots";
        assert_eq!(normalize_metric_path(path), "/api/v1/lots");
    }

    #[test]
    fn test_normalize_metric_path_multiple_ids() {
        let path = "/api/v1/lots/550e8400-e29b-41d4-a716-446655440000/slots/42";
        assert_eq!(normalize_metric_path(path), "/api/v1/lots/:id/slots/:id");
    }

    #[test]
    fn test_normalize_metric_path_root() {
        assert_eq!(normalize_metric_path("/health"), "/health");
        assert_eq!(normalize_metric_path("/metrics"), "/metrics");
    }

    // ── Wizard tests (compile unconditionally — structs are simple DTOs) ──

    #[cfg(feature = "mod-setup-wizard")]
    mod wizard_tests {
        use super::super::*;

        #[test]
        fn test_wizard_status_serialize() {
            let status = WizardStatus {
                completed: false,
                steps: vec![
                    WizardStep {
                        step: 1,
                        name: "Company Info".to_string(),
                        completed: true,
                    },
                    WizardStep {
                        step: 2,
                        name: "Create Lot".to_string(),
                        completed: false,
                    },
                ],
            };
            let json = serde_json::to_value(&status).unwrap();
            assert_eq!(json["completed"], false);
            assert_eq!(json["steps"].as_array().unwrap().len(), 2);
            assert_eq!(json["steps"][0]["completed"], true);
            assert_eq!(json["steps"][1]["completed"], false);
        }

        #[test]
        fn test_wizard_step_request_step1_deserialize() {
            let json = r#"{
                "step": 1,
                "company_name": "ParkCorp GmbH",
                "timezone": "Europe/Berlin",
                "logo_base64": "iVBOR..."
            }"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 1);
            assert_eq!(req.company_name.as_deref(), Some("ParkCorp GmbH"));
            assert_eq!(req.timezone.as_deref(), Some("Europe/Berlin"));
            assert!(req.logo_base64.is_some());
        }

        #[test]
        fn test_wizard_step_request_step2_deserialize() {
            let json = r#"{
                "step": 2,
                "lot_name": "Main Garage",
                "floor_count": 3,
                "slots_per_floor": 50
            }"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 2);
            assert_eq!(req.lot_name.as_deref(), Some("Main Garage"));
            assert_eq!(req.floor_count, Some(3));
            assert_eq!(req.slots_per_floor, Some(50));
        }

        #[test]
        fn test_wizard_step_request_step3_with_emails() {
            let json = r#"{
                "step": 3,
                "invite_emails": ["alice@test.com", "bob@test.com"]
            }"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 3);
            let emails = req.invite_emails.unwrap();
            assert_eq!(emails.len(), 2);
            assert_eq!(emails[0], "alice@test.com");
        }

        #[test]
        fn test_wizard_step_request_step4_theme() {
            let json = r#"{"step": 4, "theme": "neon"}"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 4);
            assert_eq!(req.theme.as_deref(), Some("neon"));
        }

        #[test]
        fn test_wizard_themes_list_contains_12() {
            assert_eq!(WIZARD_THEMES.len(), 12);
            assert!(WIZARD_THEMES.contains(&"classic"));
            assert!(WIZARD_THEMES.contains(&"synthwave"));
            assert!(WIZARD_THEMES.contains(&"zen"));
        }

        #[test]
        fn test_wizard_step_request_minimal() {
            let json = r#"{"step": 1}"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 1);
            assert!(req.company_name.is_none());
            assert!(req.lot_name.is_none());
            assert!(req.theme.is_none());
        }

        #[test]
        fn test_wizard_step_invalid_step_number() {
            let json = r#"{"step": 5}"#;
            let req: WizardStepRequest = serde_json::from_str(json).unwrap();
            assert_eq!(req.step, 5);
            // This would be caught at the handler level, not deserialization
        }
    }
}
