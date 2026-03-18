//! HTTP API Routes
//!
//! RESTful API for the parking system.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::audit::{AuditEntry, AuditEventType};
use crate::demo;
use crate::email;
use crate::metrics;
use crate::openapi::ApiDoc;
use crate::rate_limit::{ip_rate_limit_middleware, EndpointRateLimiters};
use crate::static_files;

/// Maximum allowed request body size: 1 MiB.
/// Prevents DoS via excessively large JSON payloads.
const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024; // 1 MiB

/// German standard VAT rate (19% — Umsatzsteuergesetz § 12 Abs. 1)
const VAT_RATE: f64 = 0.19;

use parkhub_common::models::{
    Absence, AbsencePattern, AbsenceType, Announcement, AnnouncementSeverity, GuestBooking,
    Notification, RecurringBooking, SwapRequest, SwapRequestStatus, WaitlistEntry,
};
use parkhub_common::{
    ApiResponse, Booking, BookingPricing, BookingStatus, CreateBookingRequest, CreditTransaction,
    CreditTransactionType, HandshakeRequest, HandshakeResponse, LoginResponse, PaymentStatus,
    ServerStatus, SlotStatus, User, UserRole, Vehicle, VehicleType, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::requests::VehicleRequest;
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Submodules
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) mod admin;
mod auth;
mod bookings;
mod credits;
mod export;
mod favorites;
mod lots;
pub(crate) mod push;
mod setup;
mod social;
mod users;
pub(crate) mod webhooks;
mod zones;

// Re-import handler functions so the router can reference them unqualified.
use auth::{forgot_password, login, refresh_token, register, reset_password};
use credits::{
    admin_grant_credits, admin_refill_all_credits, admin_update_user_quota, get_user_credits,
};
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_slots, list_lots,
    update_lot, update_slot,
};
use webhooks::{
    create_webhook, delete_webhook, list_webhooks, test_webhook, update_webhook,
};
use export::{admin_export_bookings_csv, admin_export_users_csv};
use favorites::{add_favorite, list_favorites, remove_favorite};
use zones::{create_zone, delete_zone, list_zones};

/// User ID extracted from auth token
#[derive(Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
}

/// Helper: verify the caller is an admin or superadmin.
/// Returns `Ok(())` on success, `Err(forbidden_response)` otherwise.
pub(crate) async fn check_admin(
    state: &crate::AppState,
    auth_user: &AuthUser,
) -> Result<(), (StatusCode, &'static str)> {
    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => Ok(()),
        _ => Err((StatusCode::FORBIDDEN, "Admin access required")),
    }
}

/// Create the API router with OpenAPI docs and metrics.
/// Returns (router, demo_state) so the demo state can be used for scheduled resets.
pub fn create_router(state: SharedState) -> (Router, demo::SharedDemoState) {
    // Initialize Prometheus metrics
    let metrics_handle = metrics::init_metrics();

    // Instantiate per-endpoint rate limiters
    let rate_limiters = EndpointRateLimiters::new();

    // Rate-limited auth routes — each sub-router gets its own per-IP limiter applied
    // via route_layer so only that specific route is affected.

    // POST /api/v1/auth/login — 5 requests per minute per IP
    let login_limiter = rate_limiters.login.clone();
    let login_route = Router::new()
        .route("/api/v1/auth/login", post(login))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(login_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/register — 3 requests per minute per IP
    let register_limiter = rate_limiters.register.clone();
    let register_route = Router::new()
        .route("/api/v1/auth/register", post(register))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(register_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/forgot-password — 3 requests per 15 minutes per IP
    let forgot_limiter = rate_limiters.forgot_password.clone();
    let forgot_route = Router::new()
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(forgot_limiter.clone(), req, next)
        }));

    // Remaining public routes (no rate limiting needed)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/auth/reset-password", post(reset_password))
        // Legal — public (DDG § 5 requires Impressum to be freely accessible)
        .route("/api/v1/legal/impressum", get(get_impressum))
        // Feature flags — public (frontend needs to know which features are enabled)
        .route("/api/v1/features", get(get_features))
        // Announcements — public (active announcements visible without auth)
        .route(
            "/api/v1/announcements/active",
            get(get_active_announcements),
        )
        // Setup wizard — only works before initial setup is completed
        .route("/api/v1/setup/status", get(setup::setup_status))
        .route("/api/v1/setup", post(setup::setup_init))
        // Public occupancy display (no auth)
        .route("/api/v1/public/occupancy", get(public_occupancy))
        .route("/api/v1/public/display", get(public_display))
        // VAPID public key (no auth — frontend needs it before login)
        .route("/api/v1/push/vapid-key", get(push::get_vapid_key));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route(
            "/api/v1/users/me",
            get(get_current_user).put(update_current_user),
        )
        .route("/api/v1/users/me/export", get(gdpr_export_data))
        .route("/api/v1/users/me/delete", delete(gdpr_delete_account))
        .route(
            "/api/v1/users/me/password",
            axum::routing::patch(change_password),
        )
        // iCal export for user's bookings
        .route("/api/v1/user/calendar.ics", get(user_calendar_ics))
        // Admin-only: retrieve any user by ID
        .route("/api/v1/users/{id}", get(get_user))
        .route("/api/v1/lots", get(list_lots).post(create_lot))
        .route(
            "/api/v1/lots/{id}",
            get(get_lot).put(update_lot).delete(delete_lot),
        )
        .route("/api/v1/lots/{id}/slots", get(get_lot_slots).post(create_slot))
        .route(
            "/api/v1/lots/{lot_id}/slots/{slot_id}",
            put(update_slot).delete(delete_slot),
        )
        // Zones (admin-only CRUD, nested under lots)
        .route(
            "/api/v1/lots/{lot_id}/zones",
            get(list_zones).post(create_zone),
        )
        .route(
            "/api/v1/lots/{lot_id}/zones/{zone_id}",
            delete(delete_zone),
        )
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route(
            "/api/v1/bookings/{id}",
            get(get_booking).delete(cancel_booking),
        )
        .route("/api/v1/bookings/{id}/invoice", get(get_booking_invoice))
        .route("/api/v1/vehicles", get(list_vehicles).post(create_vehicle))
        .route("/api/v1/vehicles/{id}", put(update_vehicle).delete(delete_vehicle))
        // Credits
        .route("/api/v1/user/credits", get(get_user_credits))
        // Favorites (user-authenticated)
        .route(
            "/api/v1/user/favorites",
            get(list_favorites).post(add_favorite),
        )
        .route("/api/v1/user/favorites/{slot_id}", delete(remove_favorite))
        // Admin-only: update Impressum settings
        .route(
            "/api/v1/admin/impressum",
            get(get_impressum_admin).put(update_impressum),
        )
        // Admin-only: user management
        .route("/api/v1/admin/users", get(admin_list_users))
        .route(
            "/api/v1/admin/users/{id}/role",
            axum::routing::patch(admin_update_user_role),
        )
        .route(
            "/api/v1/admin/users/{id}/status",
            axum::routing::patch(admin_update_user_status),
        )
        .route("/api/v1/admin/users/{id}", delete(admin_delete_user))
        // Admin-only: credits management
        .route(
            "/api/v1/admin/users/{id}/credits",
            post(admin_grant_credits),
        )
        .route(
            "/api/v1/admin/credits/refill-all",
            post(admin_refill_all_credits),
        )
        .route(
            "/api/v1/admin/users/{id}/quota",
            put(admin_update_user_quota),
        )
        // Admin-only: feature flags management
        .route(
            "/api/v1/admin/features",
            get(admin_get_features).put(admin_update_features),
        )
        // Admin-only: system settings
        .route(
            "/api/v1/admin/settings",
            get(admin_get_settings).put(admin_update_settings),
        )
        // Admin-only: all bookings
        .route("/api/v1/admin/bookings", get(admin_list_bookings))
        // Admin-only: CSV exports
        .route(
            "/api/v1/admin/users/export-csv",
            get(admin_export_users_csv),
        )
        .route(
            "/api/v1/admin/bookings/export-csv",
            get(admin_export_bookings_csv),
        )
        // Absences (user-scoped)
        .route("/api/v1/absences", get(list_absences).post(create_absence))
        .route("/api/v1/absences/team", get(list_team_absences))
        .route(
            "/api/v1/absences/pattern",
            get(get_absence_pattern).post(save_absence_pattern),
        )
        .route("/api/v1/absences/{id}", delete(delete_absence))
        // Team view
        .route("/api/v1/team/today", get(team_today))
        // Admin-only: announcements management
        .route(
            "/api/v1/admin/announcements",
            get(admin_list_announcements).post(admin_create_announcement),
        )
        .route(
            "/api/v1/admin/announcements/{id}",
            put(admin_update_announcement).delete(admin_delete_announcement),
        )
        // Notifications (user-scoped)
        .route("/api/v1/notifications", get(list_notifications))
        .route(
            "/api/v1/notifications/{id}/read",
            put(mark_notification_read),
        )
        .route(
            "/api/v1/notifications/read-all",
            post(mark_all_notifications_read),
        )
        // Waitlist
        .route("/api/v1/waitlist", get(list_waitlist).post(join_waitlist))
        .route("/api/v1/waitlist/{id}", delete(leave_waitlist))
        // Swap requests
        .route("/api/v1/swap-requests", get(list_swap_requests))
        .route(
            "/api/v1/bookings/{id}/swap-request",
            post(create_swap_request),
        )
        .route("/api/v1/swap-requests/{id}", put(update_swap_request))
        // Recurring bookings
        .route(
            "/api/v1/recurring-bookings",
            get(list_recurring_bookings).post(create_recurring_booking),
        )
        .route(
            "/api/v1/recurring-bookings/{id}",
            delete(delete_recurring_booking),
        )
        // Guest bookings
        .route("/api/v1/bookings/guest", post(create_guest_booking))
        .route(
            "/api/v1/admin/guest-bookings",
            get(admin_list_guest_bookings),
        )
        .route(
            "/api/v1/admin/guest-bookings/{id}/cancel",
            axum::routing::patch(admin_cancel_guest_booking),
        )
        // Quick book
        .route("/api/v1/bookings/quick", post(quick_book))
        // Calendar
        .route("/api/v1/calendar/events", get(calendar_events))
        // Admin reports
        .route("/api/v1/admin/stats", get(admin_stats))
        .route("/api/v1/admin/reports", get(admin_reports))
        .route("/api/v1/admin/heatmap", get(admin_heatmap))
        .route("/api/v1/admin/audit-log", get(admin_audit_log))
        // Team
        .route("/api/v1/team", get(team_list))
        // Admin-only: webhooks
        .route(
            "/api/v1/webhooks",
            get(list_webhooks).post(create_webhook),
        )
        .route(
            "/api/v1/webhooks/{id}",
            put(update_webhook).delete(delete_webhook),
        )
        .route("/api/v1/webhooks/{id}/test", post(test_webhook))
        // Push notifications
        .route("/api/v1/push/subscribe", post(push::subscribe))
        .route("/api/v1/push/unsubscribe", delete(push::unsubscribe))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Demo mode routes (no auth, in-memory state)
    let demo_state = demo::new_demo_state();
    let demo_state_ret = demo_state.clone();
    let demo_routes = Router::new()
        .route("/api/v1/demo/status", get(demo::demo_status))
        .route("/api/v1/demo/vote", post(demo::demo_vote))
        .route("/api/v1/demo/reset", post(demo::demo_reset))
        .route("/api/v1/demo/config", get(demo::demo_config))
        .layer(Extension(demo_state));

    // Clone handle for the closure
    let metrics_handle_clone = metrics_handle.clone();

    let router = Router::new()
        .merge(public_routes)
        .merge(login_route)
        .merge(register_route)
        .merge(forgot_route)
        .merge(demo_routes)
        .merge(protected_routes)
        // Prometheus metrics endpoint — protected by METRICS_TOKEN env var when set
        .route(
            "/metrics",
            get(move |req: Request<Body>| async move {
                if let Ok(expected) = std::env::var("METRICS_TOKEN") {
                    if !expected.is_empty() {
                        let authorized = req
                            .headers()
                            .get(header::AUTHORIZATION)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.strip_prefix("Bearer "))
                            .map(|token| token == expected)
                            .unwrap_or(false);
                        if !authorized {
                            return (
                                StatusCode::UNAUTHORIZED,
                                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                                "Unauthorized".to_string(),
                            );
                        }
                    }
                }
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    metrics_handle_clone.render(),
                )
            }),
        )
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Static files (web frontend) - fallback for all other routes
        .fallback(static_files::static_handler)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        // Security headers applied to every response
        .layer(axum::middleware::from_fn(security_headers_middleware))
        // Restrict request body size to prevent DoS via large payloads
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        // CORS: same-origin by default; no wildcard
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(
                    |origin: &HeaderValue, _req_parts: &axum::http::request::Parts| {
                        // Allow requests with no Origin header (same-origin, curl, mobile)
                        // and explicitly allow localhost origins during development.
                        // In production, the SPA is served from the same origin, so
                        // cross-origin requests are not expected.
                        let s = origin.to_str().unwrap_or("");
                        s.starts_with("http://localhost:")
                            || s.starts_with("https://localhost:")
                            || s.starts_with("http://127.0.0.1:")
                    },
                ))
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::PATCH,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
                .allow_credentials(false),
        );

    (router, demo_state_ret)
}

/// Middleware that adds security-related response headers to every request.
///
/// - `X-Content-Type-Options`: prevents MIME sniffing
/// - `X-Frame-Options`: prevents clickjacking
/// - `Content-Security-Policy`: restricts resource origins
/// - `Referrer-Policy`: limits referrer leakage
/// - `Permissions-Policy`: disables unneeded browser features
/// - `Strict-Transport-Security`: enforces HTTPS for 1 year including subdomains
async fn security_headers_middleware(request: Request<Body>, next: Next) -> Response {
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
            // 'unsafe-inline' removed from script-src — use nonces or hashes for
            // any inline scripts.  style-src retains 'unsafe-inline' because React
            // and similar frameworks inject critical CSS at runtime.
            "default-src 'self'; \
             script-src 'self'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self'; \
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
// AUTH MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

async fn auth_middleware(
    State(state): State<SharedState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    // Extract bearer token
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "UNAUTHORIZED",
                    "Missing or invalid authorization header",
                )),
            ));
        }
    };

    // Validate session
    let state_guard = state.read().await;
    let session = match state_guard.db.get_session(token).await {
        Ok(Some(s)) if !s.is_expired() => s,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "UNAUTHORIZED",
                    "Invalid or expired token",
                )),
            ));
        }
    };
    drop(state_guard);

    // Insert user info into request extensions
    request.extensions_mut().insert(AuthUser {
        user_id: session.user_id,
    });

    Ok(next.run(request).await)
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH & DISCOVERY
// ═══════════════════════════════════════════════════════════════════════════════

async fn health_check() -> &'static str {
    "OK"
}

/// Kubernetes liveness probe - just checks if the service is running
async fn liveness_check() -> StatusCode {
    StatusCode::OK
}

/// Kubernetes readiness probe - checks if the service can handle traffic.
///
/// Returns only a boolean `ready` field. Internal error details are logged
/// server-side but never exposed in the response body.
async fn readiness_check(State(state): State<SharedState>) -> impl IntoResponse {
    let state = state.read().await;
    match state.db.stats().await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"ready": true}))),
        Err(e) => {
            tracing::error!("Readiness check failed: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"ready": false})),
            )
        }
    }
}

async fn handshake(
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

async fn server_status(State(state): State<SharedState>) -> Json<ApiResponse<ServerStatus>> {
    let state = state.read().await;
    let db_stats = state.db.stats().await.unwrap_or(crate::db::DatabaseStats {
        users: 0,
        bookings: 0,
        parking_lots: 0,
        slots: 0,
        sessions: 0,
        vehicles: 0,
    });

    Json(ApiResponse::success(ServerStatus {
        uptime_seconds: 0,
        connected_clients: 0,
        total_users: db_stats.users as u32,
        total_bookings: db_stats.bookings as u32,
        database_size_bytes: 0,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════════
// USERS
// ═══════════════════════════════════════════════════════════════════════════════

async fn get_current_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

/// Request body for updating the current user's profile
#[derive(Debug, Deserialize)]
struct UpdateCurrentUserRequest {
    name: Option<String>,
    phone: Option<String>,
    picture: Option<String>,
}

/// `PUT /api/v1/users/me` — update the authenticated user's own profile.
///
/// Allows users to update their display name, phone number, and profile
/// picture URL. Fields not included in the request body are left unchanged.
/// Returns the updated user record (without `password_hash`).
async fn update_current_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdateCurrentUserRequest>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state_guard = state.read().await;

    let mut user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching user for update: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Apply only the fields provided in the request
    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(phone) = req.phone {
        user.phone = Some(phone);
    }
    if let Some(picture) = req.picture {
        // Validate picture URL: must be empty, or a well-formed http(s) URL
        // capped at 2048 characters to prevent abuse.
        if !picture.is_empty() {
            if picture.len() > 2048 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Picture URL must be at most 2048 characters",
                    )),
                );
            }
            if !picture.starts_with("https://") && !picture.starts_with("http://") {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Picture must be a valid HTTP or HTTPS URL",
                    )),
                );
            }
        }
        user.picture = if picture.is_empty() {
            None
        } else {
            Some(picture)
        };
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save user profile update: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update profile",
            )),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(user.id, &user.username)
        .log();

    user.password_hash = String::new();
    (StatusCode::OK, Json(ApiResponse::success(user)))
}

/// Retrieve a user by ID.
///
/// Restricted to Admin and SuperAdmin roles. Regular users must use
/// `GET /api/v1/users/me` to access their own profile.
async fn get_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    // Verify caller is an admin before exposing arbitrary user records.
    let caller = match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    match state.db.get_user(&id).await {
        Ok(Some(mut user)) => {
            user.password_hash = String::new();
            (StatusCode::OK, Json(ApiResponse::success(user)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}
// ═══════════════════════════════════════════════════════════════════════════════
// BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Booking>>> {
    let state = state.read().await;

    match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => Json(ApiResponse::success(bookings)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list bookings",
            ))
        }
    }
}

async fn create_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    // Use a WRITE lock for the entire booking creation to prevent race
    // conditions where two concurrent requests book the same slot simultaneously.
    // Both would read SlotStatus::Available, and both would succeed — leaving the
    // slot double-booked. Holding the write lock ensures only one request can
    // complete the check-and-update atomically.
    let state_guard = state.write().await;

    // Check if slot exists and is available
    let slot = match state_guard
        .db
        .get_parking_slot(&req.slot_id.to_string())
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Slot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if slot.status != SlotStatus::Available {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "SLOT_UNAVAILABLE",
                "This slot is not available",
            )),
        );
    }

    // Get or create vehicle info
    let vehicle = match state_guard
        .db
        .get_vehicle(&req.vehicle_id.to_string())
        .await
    {
        Ok(Some(v)) => {
            // Verify the vehicle belongs to the authenticated user.
            if v.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error(
                        "FORBIDDEN",
                        "Vehicle does not belong to you",
                    )),
                );
            }
            v
        }
        _ => Vehicle {
            id: req.vehicle_id,
            user_id: auth_user.user_id,
            license_plate: req.license_plate.clone(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::Car,
            is_default: false,
            created_at: Utc::now(),
        },
    };

    // Validate duration is positive before arithmetic
    if req.duration_minutes <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Duration must be positive",
            )),
        );
    }

    // Validate start_time is in the future (at least 1 minute from now)
    if req.start_time <= Utc::now() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_BOOKING_TIME",
                "Booking start time must be in the future",
            )),
        );
    }

    // ── Admin settings enforcement ─────────────────────────────────────────

    // require_vehicle: reject if no vehicle provided
    let require_vehicle = read_admin_setting(&state_guard.db, "require_vehicle").await;
    if require_vehicle == "true" && req.vehicle_id == Uuid::nil() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VEHICLE_REQUIRED",
                "A vehicle is required for booking",
            )),
        );
    }

    // license_plate_mode = "required": reject if plate is empty
    let plate_mode = read_admin_setting(&state_guard.db, "license_plate_mode").await;
    if plate_mode == "required" && req.license_plate.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "LICENSE_PLATE_REQUIRED",
                "A license plate is required for booking",
            )),
        );
    }

    // min_booking_duration_hours / max_booking_duration_hours
    let duration_hours = req.duration_minutes as f64 / 60.0;
    let min_hours: f64 = read_admin_setting(&state_guard.db, "min_booking_duration_hours")
        .await
        .parse()
        .unwrap_or(0.0);
    if min_hours > 0.0 && duration_hours < min_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_SHORT",
                format!("Minimum booking duration is {} hour(s)", min_hours),
            )),
        );
    }
    let max_hours: f64 = read_admin_setting(&state_guard.db, "max_booking_duration_hours")
        .await
        .parse()
        .unwrap_or(0.0);
    if max_hours > 0.0 && duration_hours > max_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_LONG",
                format!("Maximum booking duration is {} hour(s)", max_hours),
            )),
        );
    }

    // max_bookings_per_day: count user's bookings for the same day (0 = unlimited)
    let max_per_day: i32 = read_admin_setting(&state_guard.db, "max_bookings_per_day")
        .await
        .parse()
        .unwrap_or(0);
    if max_per_day > 0 {
        let user_bookings = state_guard
            .db
            .list_bookings_by_user(&auth_user.user_id.to_string())
            .await
            .unwrap_or_default();
        let booking_date = req.start_time.date_naive();
        let same_day_count = user_bookings
            .iter()
            .filter(|b| {
                b.start_time.date_naive() == booking_date && b.status != BookingStatus::Cancelled
            })
            .count() as i32;
        if same_day_count >= max_per_day {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::error(
                    "MAX_BOOKINGS_REACHED",
                    format!("Maximum of {} booking(s) per day reached", max_per_day),
                )),
            );
        }
    }

    // ── End admin settings enforcement ──────────────────────────────────────

    // Credits check — deduct if credits system is enabled
    let credits_enabled = state_guard
        .db
        .get_setting("credits_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        == "true";
    let credits_per_booking: i32 = state_guard
        .db
        .get_setting("credits_per_booking")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let mut booking_user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to load user")),
            );
        }
    };

    let is_admin_user =
        booking_user.role == UserRole::Admin || booking_user.role == UserRole::SuperAdmin;

    if credits_enabled && !is_admin_user && booking_user.credits_balance < credits_per_booking {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "INSUFFICIENT_CREDITS",
                "Not enough credits for this booking",
            )),
        );
    }

    // Calculate end time and pricing
    let end_time = req.start_time + Duration::minutes(req.duration_minutes as i64);

    // Look up the lot for pricing and floor name
    let lot_opt = state_guard
        .db
        .get_parking_lot(&req.lot_id.to_string())
        .await
        .ok()
        .flatten();

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map(|r| r.price)
        .unwrap_or(2.0);

    let base_price = (req.duration_minutes as f64 / 60.0) * hourly_rate;
    let tax = base_price * VAT_RATE;
    let total = base_price + tax;

    // Look up human-readable floor name from the lot's floors list
    let floor_name = if let Some(lot) = &lot_opt {
        lot.floors
            .iter()
            .find(|f| f.id == slot.floor_id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "Level 1".to_string())
    } else {
        "Level 1".to_string()
    };

    let now = Utc::now();
    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        slot_number: slot.slot_number,
        floor_name,
        vehicle,
        start_time: req.start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price,
            discount: 0.0,
            tax,
            total,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: req.notes,
    };

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create booking",
            )),
        );
    }

    // Update slot status atomically within the same write-lock scope.
    // The slot status is a critical cache of availability — if we cannot mark it
    // Reserved the slot will appear available and can be double-booked.
    let mut updated_slot = slot;
    updated_slot.status = SlotStatus::Reserved;
    if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
        tracing::error!("Failed to update slot status after booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SLOT_UPDATE_FAILED",
                "Booking created but slot status could not be updated. Please contact support.",
            )),
        );
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %booking.id,
        slot_id = %booking.slot_id,
        "Booking created"
    );

    // Deduct credits if enabled and user is not admin
    if credits_enabled && !is_admin_user {
        booking_user.credits_balance -= credits_per_booking;
        if let Err(e) = state_guard.db.save_user(&booking_user).await {
            tracing::warn!("Failed to save user credit deduction: {e}");
        }
        let tx = CreditTransaction {
            id: Uuid::new_v4(),
            user_id: auth_user.user_id,
            booking_id: Some(booking.id),
            amount: -credits_per_booking,
            transaction_type: CreditTransactionType::Deduction,
            description: Some(format!("Booking {}", booking.id)),
            granted_by: None,
            created_at: Utc::now(),
        };
        if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
            tracing::warn!("Failed to save credit transaction: {e}");
        }
    }

    // Fetch user details for audit log and confirmation email
    let user_info_opt = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten();

    if let Some(ref u) = user_info_opt {
        crate::audit::events::booking_created(auth_user.user_id, &u.username, booking.id);
    } else {
        crate::audit::events::booking_created(auth_user.user_id, "", booking.id);
    }

    // Send booking confirmation email (non-blocking, fire-and-forget).
    if let Some(u) = user_info_opt {
        let booking_id_str = booking.id.to_string();
        let floor_name = booking.floor_name.clone();
        let slot_number = booking.slot_number;
        let start_time_str = booking.start_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let end_time_str = booking.end_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let org_name = state_guard.config.organization_name.clone();
        let user_email = u.email.clone();
        let user_name = u.name.clone();
        tokio::spawn(async move {
            let email_html = email::build_booking_confirmation_email(
                &user_name,
                &booking_id_str,
                &floor_name,
                slot_number,
                &start_time_str,
                &end_time_str,
                &org_name,
            );
            if let Err(e) =
                email::send_email(&user_email, "Booking Confirmation — ParkHub", &email_html).await
            {
                tracing::warn!("Failed to send booking confirmation email: {}", e);
            }
        });
    }

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

async fn get_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state = state.read().await;

    match state.db.get_booking(&id).await {
        Ok(Some(booking)) => {
            if booking.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
            (StatusCode::OK, Json(ApiResponse::success(booking)))
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
        ),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

async fn cancel_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Use write lock so the booking status update and slot status update are
    // made while no other booking creation can interleave.
    let state_guard = state.write().await;

    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    // Only Confirmed or Pending bookings can be cancelled.
    if booking.status == BookingStatus::Cancelled {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "ALREADY_CANCELLED",
                "Booking is already cancelled",
            )),
        );
    }

    let mut updated_booking = booking.clone();
    updated_booking.status = BookingStatus::Cancelled;
    updated_booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&updated_booking).await {
        tracing::error!("Failed to update booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel booking",
            )),
        );
    }

    // Free up the slot — only restore to Available if it was Reserved.
    // Slots in Maintenance or Disabled state must remain as-is.
    if let Ok(Some(mut slot)) = state_guard
        .db
        .get_parking_slot(&booking.slot_id.to_string())
        .await
    {
        if slot.status == SlotStatus::Reserved {
            slot.status = SlotStatus::Available;
            if let Err(e) = state_guard.db.save_parking_slot(&slot).await {
                tracing::error!("Failed to restore slot status after cancellation: {}", e);
            }
        }
    }

    // Refund credits if credits system is enabled
    let credits_enabled = state_guard
        .db
        .get_setting("credits_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        == "true";
    if credits_enabled {
        let credits_per_booking: i32 = state_guard
            .db
            .get_setting("credits_per_booking")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        if let Ok(Some(mut user)) = state_guard
            .db
            .get_user(&auth_user.user_id.to_string())
            .await
        {
            if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
                user.credits_balance += credits_per_booking;
                if let Err(e) = state_guard.db.save_user(&user).await {
                    tracing::warn!("Failed to save user credit refund: {e}");
                }
                let tx = CreditTransaction {
                    id: Uuid::new_v4(),
                    user_id: auth_user.user_id,
                    booking_id: Some(booking.id),
                    amount: credits_per_booking,
                    transaction_type: CreditTransactionType::Refund,
                    description: Some(format!("Cancelled booking {}", booking.id)),
                    granted_by: None,
                    created_at: Utc::now(),
                };
                if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
                    tracing::warn!("Failed to save credit transaction: {e}");
                }
            }
        }
    }

    // Fetch username for audit log (best-effort)
    let username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::BookingCancelled)
        .user(auth_user.user_id, &username)
        .resource("booking", &id)
        .log();

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %id,
        "Booking cancelled"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// INVOICE
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/bookings/{id}/invoice`
///
/// Returns an HTML invoice for the given booking.  The authenticated user must
/// own the booking (admin users may retrieve any invoice).
///
/// The invoice includes:
/// - Company/organisation name from server config
/// - Booking reference (booking UUID)
/// - User name and email
/// - Parking lot name and slot number
/// - Start / end time and duration
/// - Itemised pricing: base price, VAT at 19% (German standard), total
#[allow(clippy::format_in_format_args)]
async fn get_booking_invoice(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let state_guard = state.read().await;

    // Fetch the booking
    let booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Booking not found".to_string(),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching booking for invoice: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Internal server error".to_string(),
            );
        }
    };

    // Ownership check — only the booking owner (or admin) may fetch the invoice
    let caller = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Access denied".to_string(),
            );
        }
    };

    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Access denied".to_string(),
        );
    }

    // Fetch user details for the invoice
    let booking_user = match state_guard.db.get_user(&booking.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => caller.clone(),
    };

    // Fetch parking lot name
    let lot_name = match state_guard
        .db
        .get_parking_lot(&booking.lot_id.to_string())
        .await
    {
        Ok(Some(lot)) => lot.name,
        _ => "Unknown Parking Lot".to_string(),
    };

    let org_name = state_guard.config.organization_name.clone();
    let company = if org_name.is_empty() {
        "ParkHub".to_string()
    } else {
        org_name
    };

    // Calculate duration in minutes
    let duration_minutes = (booking.end_time - booking.start_time).num_minutes();
    let duration_hours = duration_minutes / 60;
    let duration_mins_part = duration_minutes % 60;

    // VAT breakdown (19% German standard — Umsatzsteuergesetz § 12 Abs. 1)
    let net_price = booking.pricing.base_price;
    let vat_amount = net_price * VAT_RATE;
    let gross_total = net_price + vat_amount;

    let invoice_date = booking.created_at.format("%d.%m.%Y").to_string();
    let start_str = booking.start_time.format("%d.%m.%Y %H:%M").to_string();
    let end_str = booking.end_time.format("%d.%m.%Y %H:%M").to_string();

    let invoice_number = format!(
        "INV-{}",
        booking
            .id
            .to_string()
            .to_uppercase()
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>()
    );

    // HTML-escape all user-controlled values to prevent stored XSS
    use crate::utils::html_escape;
    let company = html_escape(&company);
    let user_name = html_escape(&booking_user.name);
    let user_email = html_escape(&booking_user.email);
    let lot_name = html_escape(&lot_name);
    let floor_name = html_escape(&booking.floor_name);
    let license_plate = html_escape(&booking.vehicle.license_plate);

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Rechnung {invoice_number}</title>
  <style>
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{ font-family: 'Helvetica Neue', Arial, sans-serif; color: #1a1a2e; background: #f8f9fa; }}
    .page {{ max-width: 800px; margin: 40px auto; background: #ffffff; padding: 60px;
             box-shadow: 0 4px 20px rgba(0,0,0,0.08); border-radius: 4px; }}
    .header {{ display: flex; justify-content: space-between; align-items: flex-start;
               border-bottom: 3px solid #1a73e8; padding-bottom: 24px; margin-bottom: 40px; }}
    .company-name {{ font-size: 28px; font-weight: 700; color: #1a73e8; }}
    .company-sub {{ font-size: 12px; color: #666; margin-top: 4px; }}
    .invoice-meta {{ text-align: right; }}
    .invoice-meta h2 {{ font-size: 22px; color: #333; }}
    .invoice-meta p {{ font-size: 13px; color: #666; margin-top: 4px; }}
    .section {{ margin-bottom: 32px; }}
    .section-title {{ font-size: 11px; font-weight: 700; color: #999; text-transform: uppercase;
                      letter-spacing: 0.1em; margin-bottom: 8px; }}
    .bill-to {{ background: #f8f9fa; padding: 16px 20px; border-radius: 4px; border-left: 3px solid #1a73e8; }}
    .bill-to p {{ font-size: 14px; line-height: 1.6; color: #333; }}
    table {{ width: 100%; border-collapse: collapse; margin-bottom: 0; }}
    thead tr {{ background: #1a73e8; color: white; }}
    thead th {{ padding: 12px 16px; text-align: left; font-size: 13px; font-weight: 600; }}
    tbody tr {{ border-bottom: 1px solid #e8ecf0; }}
    tbody tr:hover {{ background: #f8f9fa; }}
    tbody td {{ padding: 14px 16px; font-size: 14px; color: #333; }}
    .text-right {{ text-align: right; }}
    .totals {{ margin-top: 0; border-top: 2px solid #e8ecf0; }}
    .totals tr td {{ padding: 10px 16px; font-size: 14px; }}
    .totals .total-row td {{ font-size: 16px; font-weight: 700; color: #1a73e8;
                              border-top: 2px solid #1a73e8; padding-top: 14px; }}
    .badge {{ display: inline-block; padding: 4px 10px; border-radius: 20px; font-size: 12px;
              font-weight: 600; }}
    .badge-confirmed {{ background: #e8f5e9; color: #2e7d32; }}
    .footer {{ margin-top: 48px; padding-top: 24px; border-top: 1px solid #e8ecf0;
               font-size: 11px; color: #999; text-align: center; line-height: 1.6; }}
  </style>
</head>
<body>
  <div class="page">

    <!-- Header -->
    <div class="header">
      <div>
        <div class="company-name">{company}</div>
        <div class="company-sub">Parkverwaltungssystem</div>
      </div>
      <div class="invoice-meta">
        <h2>RECHNUNG</h2>
        <p><strong>{invoice_number}</strong></p>
        <p>Datum: {invoice_date}</p>
      </div>
    </div>

    <!-- Bill To -->
    <div class="section">
      <div class="section-title">Rechnungsempfänger</div>
      <div class="bill-to">
        <p><strong>{user_name}</strong></p>
        <p>{user_email}</p>
      </div>
    </div>

    <!-- Booking Details -->
    <div class="section">
      <div class="section-title">Buchungsdetails</div>
      <table>
        <thead>
          <tr>
            <th>Beschreibung</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Buchungsnummer</td>
            <td>{booking_id}</td>
          </tr>
          <tr>
            <td>Parkhaus</td>
            <td>{lot_name}</td>
          </tr>
          <tr>
            <td>Stellplatz</td>
            <td>Nr. {slot_number} &nbsp;·&nbsp; {floor_name}</td>
          </tr>
          <tr>
            <td>Fahrzeug (Kennzeichen)</td>
            <td>{license_plate}</td>
          </tr>
          <tr>
            <td>Beginn</td>
            <td>{start_str}</td>
          </tr>
          <tr>
            <td>Ende</td>
            <td>{end_str}</td>
          </tr>
          <tr>
            <td>Dauer</td>
            <td>{duration_hours} Std. {duration_mins_part} Min.</td>
          </tr>
          <tr>
            <td>Status</td>
            <td><span class="badge badge-confirmed">{status}</span></td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Pricing -->
    <div class="section">
      <div class="section-title">Rechnungsbetrag</div>
      <table>
        <thead>
          <tr>
            <th>Position</th>
            <th class="text-right">Betrag ({currency})</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Parkgebühr (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
        </tbody>
        <tbody class="totals">
          <tr>
            <td>Zwischensumme (Netto)</td>
            <td class="text-right">{net_price:.2}</td>
          </tr>
          <tr>
            <td>MwSt. 19% (§ 12 UStG)</td>
            <td class="text-right">{vat_amount:.2}</td>
          </tr>
          <tr class="total-row">
            <td>Gesamtbetrag (Brutto)</td>
            <td class="text-right">{gross_total:.2}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Footer -->
    <div class="footer">
      <p>{company} · Parkverwaltungssystem · Automatisch generierte Rechnung</p>
      <p>Diese Rechnung wurde automatisch erstellt und ist ohne Unterschrift gültig.</p>
    </div>

  </div>
</body>
</html>"#,
        invoice_number = invoice_number,
        invoice_date = invoice_date,
        company = company,
        user_name = user_name,
        user_email = user_email,
        booking_id = booking.id,
        lot_name = lot_name,
        slot_number = booking.slot_number,
        floor_name = floor_name,
        license_plate = license_plate,
        start_str = start_str,
        end_str = end_str,
        duration_hours = duration_hours,
        duration_mins_part = duration_mins_part,
        status = format!("{:?}", booking.status),
        currency = booking.pricing.currency,
        net_price = net_price,
        vat_amount = vat_amount,
        gross_total = gross_total,
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// VEHICLES
// ═══════════════════════════════════════════════════════════════════════════════

async fn list_vehicles(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Vehicle>>> {
    let state = state.read().await;

    match state
        .db
        .list_vehicles_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(vehicles) => Json(ApiResponse::success(vehicles)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list vehicles",
            ))
        }
    }
}

async fn create_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<VehicleRequest>,
) -> (StatusCode, Json<ApiResponse<Vehicle>>) {
    let vehicle = Vehicle {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        license_plate: req.license_plate,
        make: req.make,
        model: req.model,
        color: req.color,
        vehicle_type: req
            .vehicle_type
            .map(|t| serde_json::from_value(serde_json::Value::String(t)).unwrap_or_default())
            .unwrap_or_default(),
        is_default: req.is_default,
        created_at: Utc::now(),
    };

    let state_guard = state.read().await;
    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to save vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create vehicle",
            )),
        );
    }

    let username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::VehicleAdded)
        .user(auth_user.user_id, &username)
        .log();

    (StatusCode::CREATED, Json(ApiResponse::success(vehicle)))
}

/// Delete a vehicle owned by the authenticated user.
///
/// Only the vehicle's owner may delete it. Returns 404 if the vehicle does not
/// exist or 403 if it belongs to another user.
async fn delete_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Fetch the vehicle first to verify ownership.
    let vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching vehicle: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Ownership check — prevent users from deleting other users' vehicles.
    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    let username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.delete_vehicle(&id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::VehicleRemoved)
                .user(auth_user.user_id, &username)
                .log();
            tracing::info!(
                user_id = %auth_user.user_id,
                vehicle_id = %id,
                "Vehicle deleted"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete vehicle {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete vehicle",
                )),
            )
        }
    }
}

/// `PUT /api/v1/vehicles/{id}` — update vehicle details
async fn update_vehicle(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<Vehicle>>) {
    let state_guard = state.write().await;

    let mut vehicle = match state_guard.db.get_vehicle(&id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Vehicle not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Ownership check
    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    // Update fields if provided
    if let Some(plate) = req.get("license_plate").and_then(|v| v.as_str()) {
        vehicle.license_plate = plate.to_string();
    }
    if let Some(make) = req.get("make").and_then(|v| v.as_str()) {
        vehicle.make = Some(make.to_string());
    }
    if let Some(model) = req.get("model").and_then(|v| v.as_str()) {
        vehicle.model = Some(model.to_string());
    }
    if let Some(color) = req.get("color").and_then(|v| v.as_str()) {
        vehicle.color = Some(color.to_string());
    }
    if let Some(is_default) = req.get("is_default").and_then(|v| v.as_bool()) {
        vehicle.is_default = is_default;
    }

    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to update vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update vehicle")),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(vehicle)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TOKEN GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a cryptographically random access token (32 bytes, hex-encoded).
///
/// UUIDs v4 have a fixed structure that reduces effective entropy to ~122 bits.
/// Using a raw 256-bit random value is both simpler and more secure.
pub(super) fn generate_access_token() -> String {
    let mut bytes = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
    hex::encode(bytes)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PASSWORD UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

/// Hash a password using Argon2id.
///
/// Returns `Err` on the (extremely unlikely) event that hashing fails so the
/// caller can propagate a proper HTTP 500 instead of panicking.
#[allow(clippy::result_large_err)]
pub(super) fn hash_password(
    password: &str,
) -> Result<String, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| {
            tracing::error!("Password hashing failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        })
}

/// Hash a password using Argon2id, returning an `anyhow::Result`.
///
/// Used by code paths (e.g. password reset) that cannot return the typed
/// HTTP error tuple used by `hash_password`.
pub(super) fn hash_password_simple(password: &str) -> anyhow::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {}", e))
}

pub(super) fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// ═══════════════════════════════════════════════════════════════════════════════
// LEGAL / IMPRESSUM (DDG § 5)
// ═══════════════════════════════════════════════════════════════════════════════

/// DDG § 5 Impressum fields stored as settings keys with "impressum_" prefix
#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct ImpressumData {
    pub provider_name: String,
    pub provider_legal_form: String,
    pub street: String,
    pub zip_city: String,
    pub country: String,
    pub email: String,
    pub phone: String,
    pub register_court: String,
    pub register_number: String,
    pub vat_id: String,
    pub responsible_person: String,
    pub custom_text: String,
}

const IMPRESSUM_FIELDS: &[&str] = &[
    "provider_name",
    "provider_legal_form",
    "street",
    "zip_city",
    "country",
    "email",
    "phone",
    "register_court",
    "register_number",
    "vat_id",
    "responsible_person",
    "custom_text",
];

/// Public Impressum endpoint — no auth required (DDG § 5)
async fn get_impressum(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let state = state.read().await;
    let mut data = serde_json::json!({});

    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{}", field);
        let value = state
            .db
            .get_setting(&key)
            .await
            .unwrap_or(None)
            .unwrap_or_default();
        data[field] = serde_json::Value::String(value);
    }

    Json(data)
}

/// Admin: read Impressum settings (admin-only, protected).
///
/// Although the public endpoint exposes the same data, this route is kept
/// separate so admins can fetch the current values before editing them via PUT.
/// It is deliberately restricted to Admin/SuperAdmin.
async fn get_impressum_admin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state_guard = state.read().await;

    // Verify admin role.
    let caller = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
            );
        }
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    }

    let mut data = serde_json::json!({});
    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{}", field);
        let value = state_guard
            .db
            .get_setting(&key)
            .await
            .unwrap_or(None)
            .unwrap_or_default();
        data[field] = serde_json::Value::String(value);
    }

    (StatusCode::OK, Json(data))
}

/// Admin: update Impressum settings
async fn update_impressum(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Verify admin role
    let user_id_str = auth_user.user_id.to_string();
    let state_guard = state.read().await;
    let user = match state_guard.db.get_user(&user_id_str).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin required")),
            )
        }
    };
    drop(state_guard);

    if user.role != UserRole::Admin && user.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin required")),
        );
    }

    let state_guard = state.read().await;
    for field in IMPRESSUM_FIELDS {
        if let Some(serde_json::Value::String(value)) = payload.get(*field) {
            let key = format!("impressum_{}", field);
            if let Err(e) = state_guard.db.set_setting(&key, value).await {
                tracing::warn!("Failed to save impressum setting {key}: {e}");
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// GDPR — Art. 15 (Data Export) + Art. 17 (Right to Erasure)
// ═══════════════════════════════════════════════════════════════════════════════

/// GDPR Art. 15 — Export all personal data for the authenticated user
async fn gdpr_export_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let user = match state.db.get_user(&user_id).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                serde_json::to_string(&ApiResponse::<()>::error("NOT_FOUND", "User not found"))
                    .unwrap_or_default(),
            );
        }
    };

    let bookings = state
        .db
        .list_bookings_by_user(&user_id)
        .await
        .unwrap_or_default();
    let vehicles = state
        .db
        .list_vehicles_by_user(&user_id)
        .await
        .unwrap_or_default();

    // Note: password_hash is intentionally excluded from GDPR exports.
    // Exporting a password hash would allow offline brute-force attacks
    // against the user's own credential — contrary to the spirit of Art. 15.
    let export = serde_json::json!({
        "exported_at": Utc::now().to_rfc3339(),
        "gdpr_basis": "GDPR Art. 15 — Right of Access",
        "profile": {
            "id": user.id,
            "username": user.username,
            "email": user.email,
            "name": user.name,
            "phone": user.phone,
            "role": user.role,
            "created_at": user.created_at,
            "last_login": user.last_login,
            "preferences": user.preferences,
        },
        "bookings": bookings,
        "vehicles": vehicles,
    });

    let json_str = serde_json::to_string_pretty(&export).unwrap_or_default();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json_str,
    )
}

/// GDPR Art. 17 — Right to Erasure: anonymize user data, keep booking records for accounting.
/// Removes PII (name, email, username, password, vehicles) while preserving anonymized booking
/// records as required by German tax law (§ 147 AO — 10-year retention for accounting records).
async fn gdpr_delete_account(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let user_id = auth_user.user_id.to_string();
    let state_guard = state.read().await;

    // Capture username before anonymization scrubs it
    let username = state_guard
        .db
        .get_user(&user_id)
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.anonymize_user(&user_id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::UserDeleted)
                .user(auth_user.user_id, &username)
                .log();
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("GDPR anonymization failed for {}: {}", user_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to anonymize account",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — USER MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for updating a user's role
#[derive(Debug, Deserialize)]
struct UpdateUserRoleRequest {
    role: String,
}

/// Request body for updating a user's status
#[derive(Debug, Deserialize)]
struct UpdateUserStatusRequest {
    status: String,
}

use admin::AdminUserResponse;

/// `GET /api/v1/admin/users` — list all users (admin only)
async fn admin_list_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminUserResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_users().await {
        Ok(users) => {
            let response: Vec<AdminUserResponse> =
                users.iter().map(AdminUserResponse::from).collect();
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list users")),
            )
        }
    }
}

/// `PATCH /api/v1/admin/users/{id}/role` — update a user's role (admin only)
async fn admin_update_user_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRoleRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Fetch the caller to check their role for privilege escalation prevention
    let caller = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };

    // Only SuperAdmin may promote users to SuperAdmin (prevent privilege escalation)
    if req.role.as_str() == "superadmin" && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "Only a SuperAdmin can assign the SuperAdmin role",
            )),
        );
    }

    let mut user = match state_guard.db.get_user(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Parse role string
    user.role = match req.role.as_str() {
        "admin" => UserRole::Admin,
        "superadmin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    };
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user role: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    let admin_username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    AuditEntry::new(AuditEventType::RoleChanged)
        .user(auth_user.user_id, &admin_username)
        .resource("user", &id)
        .log();

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_role = %req.role,
        "Admin updated user role"
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminUserResponse::from(&user))),
    )
}

/// `PATCH /api/v1/admin/users/{id}/status` — enable or disable a user account (admin only)
async fn admin_update_user_status(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserStatusRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut user = match state_guard.db.get_user(&id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    user.is_active = req.status == "active";
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user status: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    // Revoke all sessions when a user is disabled
    if !user.is_active {
        if let Err(e) = state_guard.db.delete_sessions_by_user(user.id).await {
            tracing::error!("Failed to revoke sessions for disabled user {}: {}", id, e);
        }
    }

    tracing::info!(
        admin_id = %auth_user.user_id,
        target_user_id = %id,
        new_status = %req.status,
        "Admin updated user status"
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminUserResponse::from(&user))),
    )
}

/// `DELETE /api/v1/admin/users/{id}` — delete a user account (admin only, GDPR anonymize)
async fn admin_delete_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Prevent admin from deleting their own account via admin panel
    if id == auth_user.user_id.to_string() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "CANNOT_DELETE_SELF",
                "You cannot delete your own account",
            )),
        );
    }

    let admin_username = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .map(|u| u.username)
        .unwrap_or_default();

    match state_guard.db.anonymize_user(&id).await {
        Ok(true) => {
            AuditEntry::new(AuditEventType::UserDeleted)
                .user(auth_user.user_id, &admin_username)
                .resource("user", &id)
                .log();
            tracing::info!(
                admin_id = %auth_user.user_id,
                target_user_id = %id,
                "Admin anonymized user"
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to anonymize user {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to delete user")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN — BOOKING MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

/// Response type for admin booking listing (includes user details)
#[derive(Debug, Serialize)]
struct AdminBookingResponse {
    id: String,
    user_id: String,
    user_name: String,
    user_email: String,
    lot_id: String,
    lot_name: String,
    slot_id: String,
    slot_number: String,
    vehicle_plate: String,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    status: String,
    created_at: chrono::DateTime<Utc>,
}

/// `GET /api/v1/admin/bookings` — list all bookings (admin only)
async fn admin_list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminBookingResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = match state_guard.db.list_bookings().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list bookings",
                )),
            );
        }
    };

    // Build a response enriched with user info (best-effort: fall back to IDs if user not found)
    let mut response = Vec::with_capacity(bookings.len());
    for booking in bookings {
        let (user_name, user_email) =
            match state_guard.db.get_user(&booking.user_id.to_string()).await {
                Ok(Some(u)) => (u.name, u.email),
                _ => (booking.user_id.to_string(), String::new()),
            };

        let lot_name = match state_guard
            .db
            .get_parking_lot(&booking.lot_id.to_string())
            .await
        {
            Ok(Some(l)) => l.name,
            _ => booking.lot_id.to_string(),
        };

        response.push(AdminBookingResponse {
            id: booking.id.to_string(),
            user_id: booking.user_id.to_string(),
            user_name,
            user_email,
            lot_id: booking.lot_id.to_string(),
            lot_name,
            slot_id: booking.slot_id.to_string(),
            slot_number: booking.slot_number.to_string(),
            vehicle_plate: booking.vehicle.license_plate.clone(),
            start_time: booking.start_time,
            end_time: booking.end_time,
            status: format!("{:?}", booking.status).to_lowercase(),
            created_at: booking.created_at,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// All admin settings with their default values.
const ADMIN_SETTINGS: &[(&str, &str)] = &[
    ("company_name", "ParkHub"),
    ("use_case", "corporate"),
    ("self_registration", "true"),
    ("license_plate_mode", "optional"),
    ("display_name_format", "first_name"),
    ("max_bookings_per_day", "0"),
    ("allow_guest_bookings", "false"),
    ("auto_release_minutes", "30"),
    ("require_vehicle", "false"),
    ("waitlist_enabled", "true"),
    ("min_booking_duration_hours", "0"),
    ("max_booking_duration_hours", "0"),
    ("credits_enabled", "false"),
    ("credits_per_booking", "1"),
];

/// Read a single admin setting from DB, falling back to its default.
async fn read_admin_setting(db: &crate::db::Database, key: &str) -> String {
    if let Ok(Some(val)) = db.get_setting(key).await {
        return val;
    }
    ADMIN_SETTINGS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

/// `GET /api/v1/admin/settings` — return all settings (merged defaults + stored values)
async fn admin_get_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut data = serde_json::Map::new();
    for (key, default_val) in ADMIN_SETTINGS {
        let value = state_guard
            .db
            .get_setting(key)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| default_val.to_string());
        data.insert(key.to_string(), serde_json::Value::String(value));
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::Value::Object(data))),
    )
}

/// Validate a settings value against its allowed options.
fn validate_setting_value(key: &str, value: &str) -> Result<(), &'static str> {
    match key {
        "use_case" => {
            if !["corporate", "university", "residential", "other"].contains(&value) {
                return Err("use_case must be corporate, university, residential, or other");
            }
        }
        "self_registration"
        | "allow_guest_bookings"
        | "require_vehicle"
        | "waitlist_enabled"
        | "credits_enabled" => {
            if value != "true" && value != "false" {
                return Err("Value must be \"true\" or \"false\"");
            }
        }
        "license_plate_mode" => {
            if !["required", "optional", "disabled"].contains(&value) {
                return Err("license_plate_mode must be required, optional, or disabled");
            }
        }
        "display_name_format" => {
            if !["first_name", "full_name", "username"].contains(&value) {
                return Err("display_name_format must be first_name, full_name, or username");
            }
        }
        "max_bookings_per_day" | "auto_release_minutes" | "credits_per_booking" => {
            if value.parse::<i32>().is_err() {
                return Err("Value must be an integer");
            }
        }
        "min_booking_duration_hours" | "max_booking_duration_hours" => {
            if value.parse::<f64>().is_err() {
                return Err("Value must be a number");
            }
        }
        "company_name" => { /* any string is fine */ }
        _ => return Err("Unknown setting key"),
    }
    Ok(())
}

/// `PUT /api/v1/admin/settings` — update one or more settings (admin only)
async fn admin_update_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let obj = match payload.as_object() {
        Some(o) => o,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "Request body must be a JSON object of key-value pairs",
                )),
            );
        }
    };

    let allowed_keys: Vec<&str> = ADMIN_SETTINGS.iter().map(|(k, _)| *k).collect();
    let mut updated = serde_json::Map::new();

    for (key, val) in obj {
        if !allowed_keys.contains(&key.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_KEY",
                    format!("Unknown setting: {}", key),
                )),
            );
        }

        let value_str = match val.as_str() {
            Some(s) => s.to_string(),
            None => val.to_string().trim_matches('"').to_string(),
        };

        if let Err(msg) = validate_setting_value(key, &value_str) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("VALIDATION_ERROR", msg)),
            );
        }

        if let Err(e) = state_guard.db.set_setting(key, &value_str).await {
            tracing::error!("Failed to save setting {}: {}", key, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }

        updated.insert(key.clone(), serde_json::Value::String(value_str));
    }

    // Audit log
    if state_guard.config.audit_logging_enabled {
        let _entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .user(auth_user.user_id, "admin")
            .resource("settings", "admin_settings")
            .details(serde_json::json!({ "updated": updated }))
            .log();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::Value::Object(updated))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// FEATURE FLAGS
// ═══════════════════════════════════════════════════════════════════════════════

/// All available feature module IDs.
const FEATURE_MODULES: &[&str] = &[
    "credits",
    "absences",
    "vehicles",
    "analytics",
    "team_view",
    "booking_types",
    "invoices",
    "self_registration",
    "generative_bg",
    "micro_animations",
    "fab_quick_actions",
    "rich_empty_states",
    "onboarding_hints",
];

/// Default enabled features (business use case).
const DEFAULT_FEATURES: &[&str] = &[
    "credits",
    "absences",
    "vehicles",
    "analytics",
    "team_view",
    "booking_types",
    "invoices",
    "generative_bg",
    "micro_animations",
    "fab_quick_actions",
    "rich_empty_states",
    "onboarding_hints",
];

const SETTINGS_FEATURES_KEY: &str = "features_enabled";

/// Read enabled features from DB, falling back to defaults.
async fn read_features(db: &crate::db::Database) -> Vec<String> {
    match db.get_setting(SETTINGS_FEATURES_KEY).await {
        Ok(Some(json_str)) => serde_json::from_str::<Vec<String>>(&json_str)
            .unwrap_or_else(|_| DEFAULT_FEATURES.iter().map(|s| s.to_string()).collect()),
        _ => DEFAULT_FEATURES.iter().map(|s| s.to_string()).collect(),
    }
}

/// `GET /api/v1/features` — public endpoint returning enabled features
async fn get_features(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let enabled = read_features(&state_guard.db).await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": enabled,
            "available": FEATURE_MODULES,
        }))),
    )
}

/// `GET /api/v1/admin/features` — admin: get features with full metadata
async fn admin_get_features(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let enabled = read_features(&state_guard.db).await;

    let available: Vec<serde_json::Value> = FEATURE_MODULES
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "enabled": enabled.contains(&id.to_string()),
                "default_enabled": DEFAULT_FEATURES.contains(id),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": enabled,
            "available": available,
        }))),
    )
}

#[derive(Deserialize)]
struct UpdateFeaturesRequest {
    enabled: Vec<String>,
}

/// `PUT /api/v1/admin/features` — admin: update enabled features
async fn admin_update_features(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(body): Json<UpdateFeaturesRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.write().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Validate: only accept known feature IDs
    let valid: Vec<String> = body
        .enabled
        .iter()
        .filter(|id| FEATURE_MODULES.contains(&id.as_str()))
        .cloned()
        .collect();

    let json_str = serde_json::to_string(&valid).unwrap_or_default();
    if let Err(e) = state_guard
        .db
        .set_setting(SETTINGS_FEATURES_KEY, &json_str)
        .await
    {
        tracing::error!("Failed to save feature flags: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save features",
            )),
        );
    }

    // Audit log
    if state_guard.config.audit_logging_enabled {
        let _entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .user(auth_user.user_id, "admin")
            .resource("settings", "features_enabled")
            .details(serde_json::json!({ "features": valid }))
            .log();
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "enabled": valid,
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ABSENCES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct AbsenceQuery {
    #[serde(rename = "type")]
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/absences` — list current user's absences, optionally filtered by type
async fn list_absences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AbsenceQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Absence>>>) {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_absences_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(absences) => {
            let filtered = match query.absence_type {
                Some(ref t) => absences
                    .into_iter()
                    .filter(|a| &a.absence_type == t)
                    .collect(),
                None => absences,
            };
            (StatusCode::OK, Json(ApiResponse::success(filtered)))
        }
        Err(e) => {
            tracing::error!("Failed to list absences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list absences",
                )),
            )
        }
    }
}

#[derive(Deserialize)]
struct CreateAbsenceRequest {
    absence_type: AbsenceType,
    start_date: String,
    end_date: String,
    note: Option<String>,
}

/// Validate a date string is YYYY-MM-DD format.
fn is_valid_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// `POST /api/v1/absences` — create an absence
async fn create_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<Absence>>) {
    if !is_valid_date(&req.start_date) || !is_valid_date(&req.end_date) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Dates must be in YYYY-MM-DD format",
            )),
        );
    }

    if req.start_date > req.end_date {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "start_date must not be after end_date",
            )),
        );
    }

    let absence = Absence {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        absence_type: req.absence_type,
        start_date: req.start_date,
        end_date: req.end_date,
        note: req.note,
        source: "manual".to_string(),
        created_at: Utc::now(),
    };

    let state_guard = state.read().await;
    match state_guard.db.save_absence(&absence).await {
        Ok(()) => (StatusCode::CREATED, Json(ApiResponse::success(absence))),
        Err(e) => {
            tracing::error!("Failed to save absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to create absence",
                )),
            )
        }
    }
}

/// `DELETE /api/v1/absences/{id}` — delete own absence
async fn delete_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership
    let absence = match state_guard.db.get_absence(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Absence not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching absence: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if absence.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    match state_guard.db.delete_absence(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Absence not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete absence",
                )),
            )
        }
    }
}

/// `GET /api/v1/absences/team` — list all team absences
async fn list_team_absences(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Absence>>>) {
    let state_guard = state.read().await;
    match state_guard.db.list_absences_team().await {
        Ok(absences) => (StatusCode::OK, Json(ApiResponse::success(absences))),
        Err(e) => {
            tracing::error!("Failed to list team absences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list team absences",
                )),
            )
        }
    }
}

/// `GET /api/v1/absences/pattern` — get user's absence pattern
async fn get_absence_pattern(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Option<AbsencePattern>>>) {
    let state_guard = state.read().await;
    let key = format!("absence_pattern:{}", auth_user.user_id);
    match state_guard.db.get_setting(&key).await {
        Ok(Some(json_str)) => match serde_json::from_str::<AbsencePattern>(&json_str) {
            Ok(pattern) => (StatusCode::OK, Json(ApiResponse::success(Some(pattern)))),
            Err(_) => (StatusCode::OK, Json(ApiResponse::success(None))),
        },
        Ok(None) => (StatusCode::OK, Json(ApiResponse::success(None))),
        Err(e) => {
            tracing::error!("Failed to get absence pattern: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to get absence pattern",
                )),
            )
        }
    }
}

/// `POST /api/v1/absences/pattern` — save user's absence pattern
async fn save_absence_pattern(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(pattern): Json<AbsencePattern>,
) -> (StatusCode, Json<ApiResponse<AbsencePattern>>) {
    let state_guard = state.read().await;
    let key = format!("absence_pattern:{}", auth_user.user_id);
    let json_str = match serde_json::to_string(&pattern) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to serialize absence pattern: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Serialization error")),
            );
        }
    };

    match state_guard.db.set_setting(&key, &json_str).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(pattern))),
        Err(e) => {
            tracing::error!("Failed to save absence pattern: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to save absence pattern",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEAM VIEW
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct TeamMemberStatus {
    user_id: Uuid,
    name: String,
    username: String,
    status: String,
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/team/today` — return all users with their status today
async fn team_today(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<TeamMemberStatus>>>) {
    let state_guard = state.read().await;

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to load users")),
            );
        }
    };

    let today = Utc::now().format("%Y-%m-%d").to_string();

    let absences = state_guard
        .db
        .list_absences_team()
        .await
        .unwrap_or_default();

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut result = Vec::new();
    for user in &users {
        if !user.is_active {
            continue;
        }

        // Check for absence today
        let user_absence = absences
            .iter()
            .find(|a| a.user_id == user.id && a.start_date <= today && a.end_date >= today);

        if let Some(absence) = user_absence {
            let status = match absence.absence_type {
                AbsenceType::Homeoffice => "homeoffice",
                AbsenceType::Vacation => "vacation",
                AbsenceType::Sick => "sick",
                AbsenceType::Training => "absent",
                AbsenceType::Other => "absent",
            };
            result.push(TeamMemberStatus {
                user_id: user.id,
                name: user.name.clone(),
                username: user.username.clone(),
                status: status.to_string(),
                absence_type: Some(absence.absence_type.clone()),
            });
            continue;
        }

        // Check for booking today (confirmed or active)
        let has_booking = bookings.iter().any(|b| {
            b.user_id == user.id
                && (b.status == BookingStatus::Confirmed || b.status == BookingStatus::Active)
                && b.start_time.format("%Y-%m-%d").to_string() <= today
                && b.end_time.format("%Y-%m-%d").to_string() >= today
        });

        let status = if has_booking { "parked" } else { "available" };
        result.push(TeamMemberStatus {
            user_id: user.id,
            name: user.name.clone(),
            username: user.username.clone(),
            status: status.to_string(),
            absence_type: None,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ANNOUNCEMENTS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/announcements/active` — public, return active non-expired announcements
async fn get_active_announcements(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<Vec<Announcement>>>) {
    let state_guard = state.read().await;
    match state_guard.db.list_announcements().await {
        Ok(announcements) => {
            let now = Utc::now();
            let active: Vec<Announcement> = announcements
                .into_iter()
                .filter(|a| {
                    a.active
                        && match a.expires_at {
                            Some(exp) => exp > now,
                            None => true,
                        }
                })
                .collect();
            (StatusCode::OK, Json(ApiResponse::success(active)))
        }
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list announcements",
                )),
            )
        }
    }
}

/// `GET /api/v1/admin/announcements` — admin: list all announcements
async fn admin_list_announcements(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Announcement>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_announcements().await {
        Ok(announcements) => (StatusCode::OK, Json(ApiResponse::success(announcements))),
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list announcements",
                )),
            )
        }
    }
}

#[derive(Deserialize)]
struct CreateAnnouncementRequest {
    title: String,
    message: String,
    severity: AnnouncementSeverity,
    active: Option<bool>,
    expires_at: Option<DateTime<Utc>>,
}

/// `POST /api/v1/admin/announcements` — admin: create announcement
async fn admin_create_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateAnnouncementRequest>,
) -> (StatusCode, Json<ApiResponse<Announcement>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let announcement = Announcement {
        id: Uuid::new_v4(),
        title: req.title,
        message: req.message,
        severity: req.severity,
        active: req.active.unwrap_or(true),
        created_by: Some(auth_user.user_id),
        expires_at: req.expires_at,
        created_at: Utc::now(),
    };

    match state_guard.db.save_announcement(&announcement).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(ApiResponse::success(announcement)),
        ),
        Err(e) => {
            tracing::error!("Failed to save announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to create announcement",
                )),
            )
        }
    }
}

#[derive(Deserialize)]
struct UpdateAnnouncementRequest {
    title: Option<String>,
    message: Option<String>,
    severity: Option<AnnouncementSeverity>,
    active: Option<bool>,
    expires_at: Option<Option<DateTime<Utc>>>,
}

/// `PUT /api/v1/admin/announcements/{id}` — admin: update announcement
async fn admin_update_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAnnouncementRequest>,
) -> (StatusCode, Json<ApiResponse<Announcement>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Fetch all announcements and find by ID
    let announcements = match state_guard.db.list_announcements().await {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Failed to list announcements: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let mut announcement = match announcements.into_iter().find(|a| a.id.to_string() == id) {
        Some(a) => a,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Announcement not found")),
            );
        }
    };

    if let Some(title) = req.title {
        announcement.title = title;
    }
    if let Some(message) = req.message {
        announcement.message = message;
    }
    if let Some(severity) = req.severity {
        announcement.severity = severity;
    }
    if let Some(active) = req.active {
        announcement.active = active;
    }
    if let Some(expires_at) = req.expires_at {
        announcement.expires_at = expires_at;
    }

    match state_guard.db.save_announcement(&announcement).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(announcement))),
        Err(e) => {
            tracing::error!("Failed to update announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to update announcement",
                )),
            )
        }
    }
}

/// `DELETE /api/v1/admin/announcements/{id}` — admin: delete announcement
async fn admin_delete_announcement(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.delete_announcement(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Announcement not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete announcement: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete announcement",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/notifications` — list current user's notifications (most recent 50)
async fn list_notifications(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Notification>>>) {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(mut notifications) => {
            // Sort by created_at descending (most recent first)
            notifications.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            notifications.truncate(50);
            (StatusCode::OK, Json(ApiResponse::success(notifications)))
        }
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list notifications",
                )),
            )
        }
    }
}

/// `PUT /api/v1/notifications/{id}/read` — mark notification as read (verify ownership)
async fn mark_notification_read(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership by listing user's notifications
    let notifications = match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let owns = notifications.iter().any(|n| n.id.to_string() == id);
    if !owns {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Notification not found")),
        );
    }

    match state_guard.db.mark_notification_read(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Notification not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to mark notification read: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to mark notification read",
                )),
            )
        }
    }
}

/// `POST /api/v1/notifications/read-all` — mark all user's notifications as read
async fn mark_all_notifications_read(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<u32>>) {
    let state_guard = state.read().await;
    let notifications = match state_guard
        .db
        .list_notifications_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to list notifications: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to mark notifications read",
                )),
            );
        }
    };

    let mut count = 0u32;
    for notif in &notifications {
        if !notif.read {
            if let Ok(true) = state_guard
                .db
                .mark_notification_read(&notif.id.to_string())
                .await
            {
                count += 1;
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(count)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// WAITLIST
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/waitlist` — list current user's waitlist entries
async fn list_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<WaitlistEntry>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(entries) => Json(ApiResponse::success(entries)),
        Err(e) => {
            tracing::error!("Failed to list waitlist entries: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list waitlist entries",
            ))
        }
    }
}

/// Request body for joining the waitlist
#[derive(Debug, Deserialize)]
struct JoinWaitlistRequest {
    lot_id: Uuid,
}

/// `POST /api/v1/waitlist` — join waitlist for a lot
async fn join_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<JoinWaitlistRequest>,
) -> (StatusCode, Json<ApiResponse<WaitlistEntry>>) {
    let state_guard = state.read().await;

    // Check waitlist_enabled setting
    let waitlist_enabled = read_admin_setting(&state_guard.db, "waitlist_enabled").await;
    if waitlist_enabled != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "WAITLIST_DISABLED",
                "Waitlist is not enabled",
            )),
        );
    }

    // First-or-create: check if user already has a waitlist entry for this lot
    let existing = state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();
    if let Some(entry) = existing.iter().find(|e| e.lot_id == req.lot_id) {
        return (StatusCode::OK, Json(ApiResponse::success(entry.clone())));
    }

    let entry = WaitlistEntry {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        created_at: Utc::now(),
        notified_at: None,
    };

    if let Err(e) = state_guard.db.save_waitlist_entry(&entry).await {
        tracing::error!("Failed to save waitlist entry: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to join waitlist",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(entry)))
}

/// `DELETE /api/v1/waitlist/{id}` — leave waitlist (verify ownership)
async fn leave_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership
    match state_guard.db.get_waitlist_entry(&id).await {
        Ok(Some(entry)) => {
            if entry.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    match state_guard.db.delete_waitlist_entry(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete waitlist entry: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to leave waitlist",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SWAP REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/swap-requests` — list user's swap requests (as requester or target)
async fn list_swap_requests(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<SwapRequest>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_swap_requests_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(requests) => Json(ApiResponse::success(requests)),
        Err(e) => {
            tracing::error!("Failed to list swap requests: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list swap requests",
            ))
        }
    }
}

/// Request body for creating a swap request
#[derive(Debug, Deserialize)]
struct CreateSwapRequestBody {
    target_booking_id: Uuid,
    message: Option<String>,
}

/// `POST /api/v1/bookings/{id}/swap-request` — create a swap request
async fn create_swap_request(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(booking_id): Path<String>,
    Json(req): Json<CreateSwapRequestBody>,
) -> (StatusCode, Json<ApiResponse<SwapRequest>>) {
    let state_guard = state.read().await;

    // Get requester's booking
    let requester_booking = match state_guard.db.get_booking(&booking_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Verify ownership of requester booking
    if requester_booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "You can only create swap requests for your own bookings",
            )),
        );
    }

    // Get target booking
    let target_booking = match state_guard
        .db
        .get_booking(&req.target_booking_id.to_string())
        .await
    {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Target booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Validate: different users
    if requester_booking.user_id == target_booking.user_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_SWAP",
                "Cannot swap with your own booking",
            )),
        );
    }

    // Validate: same lot
    if requester_booking.lot_id != target_booking.lot_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_SWAP",
                "Bookings must be in the same lot",
            )),
        );
    }

    let swap_request = SwapRequest {
        id: Uuid::new_v4(),
        requester_booking_id: requester_booking.id,
        target_booking_id: target_booking.id,
        requester_id: auth_user.user_id,
        target_id: target_booking.user_id,
        status: SwapRequestStatus::Pending,
        message: req.message,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_swap_request(&swap_request).await {
        tracing::error!("Failed to save swap request: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create swap request",
            )),
        );
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(swap_request)),
    )
}

/// Request body for accepting/declining a swap request
#[derive(Debug, Deserialize)]
struct UpdateSwapRequestBody {
    action: String,
}

/// `PUT /api/v1/swap-requests/{id}` — accept or decline a swap request
async fn update_swap_request(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSwapRequestBody>,
) -> (StatusCode, Json<ApiResponse<SwapRequest>>) {
    // Use write lock for atomic swap if accepting
    let state_guard = state.write().await;

    let mut swap = match state_guard.db.get_swap_request(&id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Swap request not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Only the target user can accept/decline
    if swap.target_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "Only the target user can respond to this swap request",
            )),
        );
    }

    if swap.status != SwapRequestStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "ALREADY_RESOLVED",
                "This swap request has already been resolved",
            )),
        );
    }

    match req.action.as_str() {
        "accept" => {
            // Get both bookings
            let mut requester_booking = match state_guard
                .db
                .get_booking(&swap.requester_booking_id.to_string())
                .await
            {
                Ok(Some(b)) => b,
                _ => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(
                            "SERVER_ERROR",
                            "Requester booking not found",
                        )),
                    );
                }
            };

            let mut target_booking = match state_guard
                .db
                .get_booking(&swap.target_booking_id.to_string())
                .await
            {
                Ok(Some(b)) => b,
                _ => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(
                            "SERVER_ERROR",
                            "Target booking not found",
                        )),
                    );
                }
            };

            // Swap slot_ids between the two bookings
            std::mem::swap(&mut requester_booking.slot_id, &mut target_booking.slot_id);
            std::mem::swap(
                &mut requester_booking.slot_number,
                &mut target_booking.slot_number,
            );
            std::mem::swap(
                &mut requester_booking.floor_name,
                &mut target_booking.floor_name,
            );
            let now = Utc::now();
            requester_booking.updated_at = now;
            target_booking.updated_at = now;

            if let Err(e) = state_guard.db.save_booking(&requester_booking).await {
                tracing::error!("Failed to save requester booking during swap: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to perform swap")),
                );
            }
            if let Err(e) = state_guard.db.save_booking(&target_booking).await {
                tracing::error!("Failed to save target booking during swap: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to perform swap")),
                );
            }

            swap.status = SwapRequestStatus::Accepted;
        }
        "decline" => {
            swap.status = SwapRequestStatus::Declined;
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_ACTION",
                    "Action must be 'accept' or 'decline'",
                )),
            );
        }
    }

    if let Err(e) = state_guard.db.save_swap_request(&swap).await {
        tracing::error!("Failed to update swap request: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update swap request",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(swap)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// RECURRING BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/recurring-bookings` — list user's recurring bookings
async fn list_recurring_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<RecurringBooking>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => Json(ApiResponse::success(bookings)),
        Err(e) => {
            tracing::error!("Failed to list recurring bookings: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list recurring bookings",
            ))
        }
    }
}

/// Request body for creating a recurring booking
#[derive(Debug, Deserialize)]
struct CreateRecurringBookingRequest {
    lot_id: Uuid,
    slot_id: Option<Uuid>,
    days_of_week: Vec<u8>,
    start_date: String,
    end_date: Option<String>,
    start_time: String,
    end_time: String,
    vehicle_plate: Option<String>,
}

/// `POST /api/v1/recurring-bookings` — create a recurring booking
async fn create_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateRecurringBookingRequest>,
) -> (StatusCode, Json<ApiResponse<RecurringBooking>>) {
    let state_guard = state.read().await;

    let booking = RecurringBooking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        days_of_week: req.days_of_week,
        start_date: req.start_date,
        end_date: req.end_date,
        start_time: req.start_time,
        end_time: req.end_time,
        vehicle_plate: req.vehicle_plate,
        active: true,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_recurring_booking(&booking).await {
        tracing::error!("Failed to save recurring booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create recurring booking",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

/// `DELETE /api/v1/recurring-bookings/{id}` — delete recurring booking (verify ownership)
async fn delete_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Check ownership via listing user's recurring bookings
    let user_bookings = state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let id_uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid ID format")),
            );
        }
    };

    if !user_bookings.iter().any(|b| b.id == id_uuid) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    match state_guard.db.delete_recurring_booking(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NOT_FOUND",
                "Recurring booking not found",
            )),
        ),
        Err(e) => {
            tracing::error!("Failed to delete recurring booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete recurring booking",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GUEST BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for creating a guest booking
#[derive(Debug, Deserialize)]
struct CreateGuestBookingRequest {
    lot_id: Uuid,
    slot_id: Uuid,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>,
    guest_name: String,
    guest_email: Option<String>,
}

/// Generate an 8-character random alphanumeric guest code
fn generate_guest_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// `POST /api/v1/bookings/guest` — create a guest booking
async fn create_guest_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateGuestBookingRequest>,
) -> (StatusCode, Json<ApiResponse<GuestBooking>>) {
    let state_guard = state.read().await;

    // Check allow_guest_bookings setting
    let allowed = read_admin_setting(&state_guard.db, "allow_guest_bookings").await;
    if allowed != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "GUEST_BOOKINGS_DISABLED",
                "Guest bookings are not enabled",
            )),
        );
    }

    let guest_booking = GuestBooking {
        id: Uuid::new_v4(),
        created_by: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: req.slot_id,
        guest_name: req.guest_name,
        guest_email: req.guest_email,
        guest_code: generate_guest_code(),
        start_time: req.start_time,
        end_time: req.end_time,
        vehicle_plate: None,
        status: BookingStatus::Confirmed,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_guest_booking(&guest_booking).await {
        tracing::error!("Failed to save guest booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create guest booking",
            )),
        );
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(guest_booking)),
    )
}

/// `GET /api/v1/admin/guest-bookings` — admin: list all guest bookings
async fn admin_list_guest_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<GuestBooking>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_guest_bookings().await {
        Ok(bookings) => (StatusCode::OK, Json(ApiResponse::success(bookings))),
        Err(e) => {
            tracing::error!("Failed to list guest bookings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list guest bookings",
                )),
            )
        }
    }
}

/// `PATCH /api/v1/admin/guest-bookings/{id}/cancel` — admin: cancel a guest booking
async fn admin_cancel_guest_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<GuestBooking>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut booking = match state_guard.db.get_guest_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Guest booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    booking.status = BookingStatus::Cancelled;

    if let Err(e) = state_guard.db.save_guest_booking(&booking).await {
        tracing::error!("Failed to cancel guest booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to cancel guest booking",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUICK BOOK
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for quick booking
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct QuickBookRequest {
    lot_id: Uuid,
    date: Option<String>,
    booking_type: Option<String>,
}

/// `POST /api/v1/bookings/quick` — quick book with auto-assigned slot
async fn quick_book(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<QuickBookRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.write().await;

    // Find first available slot in the lot
    let slots = match state_guard
        .db
        .list_slots_by_lot(&req.lot_id.to_string())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list slots: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list slots")),
            );
        }
    };

    let available_slot = match slots.iter().find(|s| s.status == SlotStatus::Available) {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::error(
                    "NO_SLOTS_AVAILABLE",
                    "No available slots in this lot",
                )),
            );
        }
    };

    // Get user's default vehicle (or first vehicle)
    let vehicles = state_guard
        .db
        .list_vehicles_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let vehicle = vehicles
        .iter()
        .find(|v| v.is_default)
        .or_else(|| vehicles.first())
        .cloned()
        .unwrap_or(Vehicle {
            id: Uuid::new_v4(),
            user_id: auth_user.user_id,
            license_plate: String::new(),
            make: None,
            model: None,
            color: None,
            vehicle_type: VehicleType::Car,
            is_default: false,
            created_at: Utc::now(),
        });

    // Determine booking times based on type
    let booking_type = req.booking_type.as_deref().unwrap_or("full_day");
    let now = Utc::now();
    let (start_time, end_time) = match booking_type {
        "half_day_am" => {
            let start = now + Duration::minutes(1);
            let end = start + Duration::hours(4);
            (start, end)
        }
        "half_day_pm" => {
            let start = now + Duration::minutes(1);
            let end = start + Duration::hours(4);
            (start, end)
        }
        _ => {
            // full_day default: 8 hours
            let start = now + Duration::minutes(1);
            let end = start + Duration::hours(8);
            (start, end)
        }
    };

    // Look up floor name and pricing from the lot
    let lot_opt = state_guard
        .db
        .get_parking_lot(&req.lot_id.to_string())
        .await
        .ok()
        .flatten();

    let floor_name = if let Some(lot) = &lot_opt {
        lot.floors
            .iter()
            .find(|f| f.id == available_slot.floor_id)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| "Level 1".to_string())
    } else {
        "Level 1".to_string()
    };

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map(|r| r.price)
        .unwrap_or(2.0);

    let base_price = ((end_time - start_time).num_minutes() as f64 / 60.0) * hourly_rate;
    let tax = base_price * VAT_RATE;
    let total = base_price + tax;

    let booking = Booking {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        slot_id: available_slot.id,
        slot_number: available_slot.slot_number,
        floor_name,
        vehicle,
        start_time,
        end_time,
        status: BookingStatus::Confirmed,
        pricing: BookingPricing {
            base_price,
            discount: 0.0,
            tax,
            total,
            currency: "EUR".to_string(),
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: Some(format!("Quick book ({})", booking_type)),
    };

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save quick booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create booking",
            )),
        );
    }

    // Update slot status
    let mut updated_slot = available_slot;
    updated_slot.status = SlotStatus::Reserved;
    if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
        tracing::error!("Failed to update slot status: {}", e);
    }

    tracing::info!(
        user_id = %auth_user.user_id,
        booking_id = %booking.id,
        slot_id = %booking.slot_id,
        "Quick booking created"
    );

    (StatusCode::CREATED, Json(ApiResponse::success(booking)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// CALENDAR
// ═══════════════════════════════════════════════════════════════════════════════

/// Query params for calendar events
#[derive(Debug, Deserialize)]
struct CalendarQuery {
    from: Option<String>,
    to: Option<String>,
}

/// Calendar event response
#[derive(Debug, Serialize)]
struct CalendarEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    title: String,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lot_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

/// `GET /api/v1/calendar/events` — return user's bookings + absences as calendar events
async fn calendar_events(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<CalendarQuery>,
) -> Json<ApiResponse<Vec<CalendarEvent>>> {
    let state_guard = state.read().await;
    let mut events = Vec::new();

    // Parse date range for filtering
    let from_date = query
        .from
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to_date = query
        .to
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    // Bookings as events
    if let Ok(bookings) = state_guard
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        for b in bookings {
            // Filter by date range if provided
            if let Some(from) = from_date {
                if b.start_time.date_naive() < from {
                    continue;
                }
            }
            if let Some(to) = to_date {
                if b.start_time.date_naive() > to {
                    continue;
                }
            }

            events.push(CalendarEvent {
                id: b.id.to_string(),
                event_type: "booking".to_string(),
                title: format!("Parking - Slot {}", b.slot_number),
                start: b.start_time,
                end: b.end_time,
                lot_name: Some(b.floor_name.clone()),
                slot_number: Some(b.slot_number),
                status: Some(format!("{:?}", b.status).to_lowercase()),
            });
        }
    }

    // Absences as events
    if let Ok(absences) = state_guard
        .db
        .list_absences_by_user(&auth_user.user_id.to_string())
        .await
    {
        for a in absences {
            let start = chrono::NaiveDate::parse_from_str(&a.start_date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
            let end = chrono::NaiveDate::parse_from_str(&a.end_date, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(23, 59, 59))
                .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

            if let (Some(start_dt), Some(end_dt)) = (start, end) {
                // Filter by date range
                if let Some(from) = from_date {
                    if end_dt.date_naive() < from {
                        continue;
                    }
                }
                if let Some(to) = to_date {
                    if start_dt.date_naive() > to {
                        continue;
                    }
                }

                let type_label = format!("{:?}", a.absence_type);
                events.push(CalendarEvent {
                    id: a.id.to_string(),
                    event_type: "absence".to_string(),
                    title: type_label,
                    start: start_dt,
                    end: end_dt,
                    lot_name: None,
                    slot_number: None,
                    status: None,
                });
            }
        }
    }

    // Sort by start time
    events.sort_by(|a, b| a.start.cmp(&b.start));

    Json(ApiResponse::success(events))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN REPORTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Dashboard stats response
#[derive(Debug, Serialize)]
struct AdminStatsResponse {
    total_users: u64,
    total_lots: u64,
    total_slots: u64,
    total_bookings: u64,
    active_bookings: u64,
    occupancy_percent: f64,
}

/// `GET /api/v1/admin/stats` — dashboard stats
async fn admin_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<AdminStatsResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db_stats = state_guard
        .db
        .stats()
        .await
        .unwrap_or(crate::db::DatabaseStats {
            users: 0,
            bookings: 0,
            parking_lots: 0,
            slots: 0,
            sessions: 0,
            vehicles: 0,
        });

    // Count active bookings
    let active_bookings = state_guard
        .db
        .list_bookings()
        .await
        .map(|bookings| {
            bookings
                .iter()
                .filter(|b| {
                    b.status == BookingStatus::Confirmed || b.status == BookingStatus::Active
                })
                .count() as u64
        })
        .unwrap_or(0);

    let occupancy = if db_stats.slots > 0 {
        (active_bookings as f64 / db_stats.slots as f64) * 100.0
    } else {
        0.0
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminStatsResponse {
            total_users: db_stats.users,
            total_lots: db_stats.parking_lots,
            total_slots: db_stats.slots,
            total_bookings: db_stats.bookings,
            active_bookings,
            occupancy_percent: (occupancy * 100.0).round() / 100.0,
        })),
    )
}

/// Query params for reports
#[derive(Debug, Deserialize)]
struct ReportsQuery {
    days: Option<i64>,
}

/// Booking stats by day
#[derive(Debug, Serialize)]
struct DailyBookingStat {
    date: String,
    count: usize,
}

/// `GET /api/v1/admin/reports` — booking stats by day for last N days
async fn admin_reports(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ReportsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<DailyBookingStat>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let days = query.days.unwrap_or(30);
    let cutoff = Utc::now() - Duration::days(days);

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // Group by date
    let mut by_date: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *by_date.entry(date).or_insert(0) += 1;
        }
    }

    let stats: Vec<DailyBookingStat> = by_date
        .into_iter()
        .map(|(date, count)| DailyBookingStat { date, count })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(stats)))
}

/// Heatmap cell: booking count by weekday x hour
#[derive(Debug, Serialize)]
struct HeatmapCell {
    weekday: u32,
    hour: u32,
    count: usize,
}

/// `GET /api/v1/admin/heatmap` — booking counts by weekday x hour
async fn admin_heatmap(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<HeatmapCell>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // Build 7x24 grid (weekday 0=Mon .. 6=Sun, hour 0..23)
    let mut grid = [[0usize; 24]; 7];
    for b in &bookings {
        let weekday = b.start_time.weekday().num_days_from_monday() as usize;
        let hour = b.start_time.hour() as usize;
        if weekday < 7 && hour < 24 {
            grid[weekday][hour] += 1;
        }
    }

    let cells: Vec<HeatmapCell> = grid
        .iter()
        .enumerate()
        .flat_map(|(wd, hours)| {
            hours
                .iter()
                .enumerate()
                .map(move |(h, &count)| HeatmapCell {
                    weekday: wd as u32,
                    hour: h as u32,
                    count,
                })
        })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(cells)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIT LOG
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/audit-log` — list recent audit entries
async fn admin_audit_log(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<ApiResponse<Vec<crate::db::AuditLogEntry>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100usize)
        .min(500);

    match state_guard.db.list_audit_log(limit).await {
        Ok(entries) => (StatusCode::OK, Json(ApiResponse::success(entries))),
        Err(e) => {
            tracing::error!("Failed to list audit log: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list audit log")),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEAM LIST
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
struct TeamMember {
    id: Uuid,
    name: String,
    username: String,
    role: String,
    is_active: bool,
}

/// `GET /api/v1/team` — list all team members (simplified view)
async fn team_list(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<TeamMember>>> {
    let state_guard = state.read().await;

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let members: Vec<TeamMember> = users
        .into_iter()
        .filter(|u| u.is_active)
        .map(|u| TeamMember {
            id: u.id,
            name: u.name,
            username: u.username,
            role: format!("{:?}", u.role).to_lowercase(),
            is_active: u.is_active,
        })
        .collect();

    Json(ApiResponse::success(members))
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHANGE PASSWORD
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for password change
#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

/// `PATCH /api/v1/users/me/password` — authenticated user changes their own password
async fn change_password(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ChangePasswordRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Validate new password length
    if req.new_password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "New password must be at least 8 characters",
            )),
        );
    }

    let state_guard = state.read().await;
    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Verify current password
    if !verify_password(&req.current_password, &user.password_hash) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error(
                "INVALID_PASSWORD",
                "Current password is incorrect",
            )),
        );
    }

    // Hash new password
    let new_hash = match hash_password_simple(&req.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Password hashing failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Update user
    let mut updated_user = user;
    updated_user.password_hash = new_hash;
    updated_user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&updated_user).await {
        tracing::error!("Failed to save user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update password")),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(())),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// iCAL EXPORT
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/calendar.ics` — export user's bookings as iCal
async fn user_calendar_ics(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;

    let bookings = match state_guard
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to list bookings for iCal: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                "Failed to generate calendar".to_string(),
            );
        }
    };

    let mut ical = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//ParkHub//EN\r\n");

    for b in &bookings {
        // Resolve lot name (best-effort)
        let lot_name = match state_guard
            .db
            .get_parking_lot(&b.lot_id.to_string())
            .await
        {
            Ok(Some(l)) => l.name,
            _ => "Unknown Lot".to_string(),
        };

        let dtstart = b.start_time.format("%Y%m%dT%H%M%SZ");
        let dtend = b.end_time.format("%Y%m%dT%H%M%SZ");

        ical.push_str("BEGIN:VEVENT\r\n");
        ical.push_str(&format!("UID:{}@parkhub\r\n", b.id));
        ical.push_str(&format!("DTSTART:{}\r\n", dtstart));
        ical.push_str(&format!("DTEND:{}\r\n", dtend));
        ical.push_str(&format!(
            "SUMMARY:Parking - {} - Slot {}\r\n",
            lot_name, b.slot_number
        ));
        ical.push_str("END:VEVENT\r\n");
    }

    ical.push_str("END:VCALENDAR\r\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        ical,
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC OCCUPANCY
// ═══════════════════════════════════════════════════════════════════════════════

/// Occupancy info for a single lot
#[derive(Debug, Serialize)]
struct LotOccupancy {
    lot_id: String,
    lot_name: String,
    total_slots: i32,
    occupied_slots: i32,
    available_slots: i32,
}

/// `GET /api/v1/public/occupancy` — public JSON occupancy data
async fn public_occupancy(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<Vec<LotOccupancy>>>) {
    let state_guard = state.read().await;

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots for occupancy: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get occupancy")),
            );
        }
    };

    // Count active bookings per lot
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut occupancy = Vec::with_capacity(lots.len());
    for lot in &lots {
        let occupied = bookings
            .iter()
            .filter(|b| {
                b.lot_id == lot.id
                    && b.start_time <= now
                    && b.end_time >= now
                    && matches!(
                        b.status,
                        parkhub_common::BookingStatus::Confirmed
                            | parkhub_common::BookingStatus::Active
                    )
            })
            .count() as i32;

        let available = (lot.total_slots - occupied).max(0);

        occupancy.push(LotOccupancy {
            lot_id: lot.id.to_string(),
            lot_name: lot.name.clone(),
            total_slots: lot.total_slots,
            occupied_slots: occupied,
            available_slots: available,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(occupancy)))
}

/// `GET /api/v1/public/display` — simplified HTML for parking displays
async fn public_display(
    State(state): State<SharedState>,
) -> impl axum::response::IntoResponse {
    let state_guard = state.read().await;

    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut html = String::from(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<meta http-equiv="refresh" content="30">
<title>ParkHub — Parking Availability</title>
<style>
  body { font-family: system-ui, sans-serif; margin: 0; padding: 2rem; background: #1a1a2e; color: #eee; }
  h1 { text-align: center; margin-bottom: 2rem; }
  .lots { display: flex; flex-wrap: wrap; gap: 1.5rem; justify-content: center; }
  .lot { background: #16213e; border-radius: 12px; padding: 1.5rem 2rem; min-width: 220px; text-align: center; }
  .lot-name { font-size: 1.2rem; font-weight: 600; margin-bottom: 0.5rem; }
  .available { font-size: 3rem; font-weight: 700; }
  .available.green { color: #4ade80; }
  .available.yellow { color: #facc15; }
  .available.red { color: #f87171; }
  .label { font-size: 0.9rem; color: #94a3b8; }
</style>
</head>
<body>
<h1>Parking Availability</h1>
<div class="lots">
"#,
    );

    for lot in &lots {
        let occupied = bookings
            .iter()
            .filter(|b| {
                b.lot_id == lot.id
                    && b.start_time <= now
                    && b.end_time >= now
                    && matches!(
                        b.status,
                        parkhub_common::BookingStatus::Confirmed
                            | parkhub_common::BookingStatus::Active
                    )
            })
            .count() as i32;

        let available = (lot.total_slots - occupied).max(0);
        let pct = if lot.total_slots > 0 {
            (available as f64 / lot.total_slots as f64) * 100.0
        } else {
            0.0
        };
        let color_class = if pct > 30.0 {
            "green"
        } else if pct > 10.0 {
            "yellow"
        } else {
            "red"
        };

        html.push_str(&format!(
            r#"<div class="lot">
  <div class="lot-name">{}</div>
  <div class="available {}">{}</div>
  <div class="label">of {} available</div>
</div>
"#,
            lot.name, color_class, available, lot.total_slots
        ));
    }

    html.push_str("</div>\n</body>\n</html>\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}
