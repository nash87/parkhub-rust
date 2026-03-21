//! HTTP API Routes
//!
//! `RESTful` API for the parking system.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Datelike, TimeDelta, Timelike, Utc};
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::Instrument;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use base64::Engine;

use crate::audit::{AuditEntry, AuditEventType};
use crate::utils::html_escape;
use crate::demo;
use crate::email;
use crate::metrics;
use crate::openapi::ApiDoc;
use crate::rate_limit::{ip_rate_limit_middleware, EndpointRateLimiters};
use crate::static_files;

/// Maximum allowed request body size: 4 MiB.
/// Raised from 1 MiB to accommodate base64-encoded vehicle photos (max 2 MB raw
/// ≈ 2.7 MB base64 + JSON envelope).  Normal API payloads remain well under this.
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024; // 4 MiB

/// Maximum raw photo size in bytes (2 MB).
const MAX_PHOTO_BYTES: usize = 2 * 1024 * 1024;


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

pub mod admin;
pub mod auth;
pub mod bookings;
pub mod credits;
pub mod export;
pub mod favorites;
pub mod lots;
pub mod payments;
pub mod push;
pub mod qr;
pub mod recommendations;
pub mod setup;
pub mod social;
pub mod translations;
pub mod users;
pub mod webhooks;
pub mod notifications;
pub mod ws;
pub mod zones;

// Re-import handler functions so the router can reference them unqualified.
use auth::{forgot_password, login, refresh_token, register, reset_password};
use credits::{
    admin_grant_credits, admin_refill_all_credits, admin_update_user_quota, get_user_credits,
};
use export::{admin_export_bookings_csv, admin_export_revenue_csv, admin_export_users_csv};
use favorites::{add_favorite, list_favorites, remove_favorite};
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_slots, list_lots,
    update_lot, update_slot,
};
use recommendations::get_recommendations;
use translations::{
    create_proposal, get_proposal, list_overrides, list_proposals, review_proposal,
    vote_on_proposal,
};
use webhooks::{create_webhook, delete_webhook, list_webhooks, test_webhook, update_webhook};
use zones::{create_zone, delete_zone, list_zones};
use bookings::{
    booking_checkin, calendar_events, cancel_booking, create_booking, create_guest_booking,
    create_recurring_booking, delete_recurring_booking, get_booking, get_booking_invoice,
    list_bookings, list_recurring_bookings, quick_book,
};
use users::{
    change_password, gdpr_delete_account, gdpr_export_data, get_current_user, get_user,
    team_list, update_current_user, update_user_preferences, user_calendar_ics, user_stats,
    get_user_preferences,
};
use admin::{
    admin_audit_log, admin_cancel_guest_booking, admin_create_announcement,
    admin_dashboard_charts, admin_delete_announcement, admin_delete_user,
    admin_get_auto_release, admin_get_email_settings, admin_get_features,
    admin_get_privacy, admin_get_settings, admin_get_use_case, admin_heatmap,
    admin_list_announcements, admin_list_bookings, admin_list_guest_bookings,
    admin_list_users, admin_reports, admin_reset, admin_stats, admin_update_announcement,
    admin_update_auto_release, admin_update_email_settings, admin_update_features,
    admin_update_privacy, admin_update_settings, admin_update_user, admin_update_user_role,
    admin_update_user_status, get_features, get_impressum, get_impressum_admin,
    get_public_theme, update_impressum,
};
use social::{
    create_absence, delete_absence, get_absence_pattern, join_waitlist, leave_waitlist,
    list_absences, list_swap_requests, list_team_absences, list_waitlist,
    save_absence_pattern, team_today, create_swap_request, update_swap_request,
};
use notifications::{
    get_active_announcements, list_notifications, mark_all_notifications_read,
    mark_notification_read,
};


/// User ID extracted from auth token
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
}

/// Helper: verify the caller is an admin or superadmin.
/// Returns `Ok(())` on success, `Err(forbidden_response)` otherwise.
pub async fn check_admin(
    state: &crate::AppState,
    auth_user: &AuthUser,
) -> Result<(), (StatusCode, &'static str)> {
    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => Ok(()),
        _ => Err((StatusCode::FORBIDDEN, "Admin access required")),
    }
}

/// Create the API router with `OpenAPI` docs and metrics.
/// Returns (router, `demo_state`) so the demo state can be used for scheduled resets.
pub fn create_router(state: SharedState) -> (Router, demo::SharedDemoState) {
    // Initialize Prometheus metrics
    let metrics_handle = metrics::init_metrics();

    // Instantiate per-endpoint rate limiters
    let rate_limiters = EndpointRateLimiters::new();
    let global_limiter = rate_limiters.general.clone();

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

    // GET /api/v1/bookings/:id/qr — 10 requests per minute per IP (QR pass generation)
    let qr_limiter = rate_limiters.qr_pass.clone();
    let qr_route = Router::new()
        .route("/api/v1/bookings/{id}/qr", get(qr::booking_qr_code))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(qr_limiter.clone(), req, next)
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
        // Theme — public (frontend needs theme before auth for login page styling)
        .route("/api/v1/theme", get(get_public_theme))
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
        .route("/api/v1/push/vapid-key", get(push::get_vapid_key))
        // WebSocket real-time events
        .route("/api/v1/ws", get(ws::ws_handler));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route(
            "/api/v1/users/me",
            get(get_current_user).put(update_current_user),
        )
        // Alias: frontend may call /api/v1/me — keep both paths working
        .route("/api/v1/me", get(get_current_user).put(update_current_user))
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
        .route(
            "/api/v1/lots/{id}/slots",
            get(get_lot_slots).post(create_slot),
        )
        .route(
            "/api/v1/lots/{lot_id}/slots/{slot_id}",
            put(update_slot).delete(delete_slot),
        )
        // QR code for lot
        .route("/api/v1/lots/{id}/qr", get(lot_qr_code))
        // Zones (admin-only CRUD, nested under lots)
        .route(
            "/api/v1/lots/{lot_id}/zones",
            get(list_zones).post(create_zone),
        )
        .route("/api/v1/lots/{lot_id}/zones/{zone_id}", delete(delete_zone))
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route(
            "/api/v1/bookings/{id}",
            get(get_booking).delete(cancel_booking),
        )
        .route("/api/v1/bookings/{id}/invoice", get(get_booking_invoice))
        // Smart parking recommendations
        .route("/api/v1/bookings/recommendations", get(get_recommendations))
        .route("/api/v1/vehicles", get(list_vehicles).post(create_vehicle))
        // City codes must come before {id} to avoid parameter capture
        .route("/api/v1/vehicles/city-codes", get(vehicle_city_codes))
        .route(
            "/api/v1/vehicles/{id}",
            put(update_vehicle).delete(delete_vehicle),
        )
        // Vehicle photos
        .route(
            "/api/v1/vehicles/{id}/photo",
            post(upload_vehicle_photo).get(get_vehicle_photo),
        )
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
        .route("/api/v1/admin/settings/use-case", get(admin_get_use_case))
        // Admin-only: all bookings
        .route("/api/v1/admin/bookings", get(admin_list_bookings))
        // Admin-only: CSV exports
        .route(
            "/api/v1/admin/export/users",
            get(admin_export_users_csv),
        )
        .route(
            "/api/v1/admin/export/bookings",
            get(admin_export_bookings_csv),
        )
        .route(
            "/api/v1/admin/export/revenue",
            get(admin_export_revenue_csv),
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
        // Admin reports & dashboard
        .route("/api/v1/admin/stats", get(admin_stats))
        .route("/api/v1/admin/reports", get(admin_reports))
        .route("/api/v1/admin/heatmap", get(admin_heatmap))
        .route(
            "/api/v1/admin/dashboard/charts",
            get(admin_dashboard_charts),
        )
        .route("/api/v1/admin/audit-log", get(admin_audit_log))
        // Team
        .route("/api/v1/team", get(team_list))
        // Admin-only: webhooks
        .route("/api/v1/webhooks", get(list_webhooks).post(create_webhook))
        .route(
            "/api/v1/webhooks/{id}",
            put(update_webhook).delete(delete_webhook),
        )
        .route("/api/v1/webhooks/{id}/test", post(test_webhook))
        // Push notifications
        .route("/api/v1/push/subscribe", post(push::subscribe))
        .route("/api/v1/push/unsubscribe", delete(push::unsubscribe))
        // User stats & preferences
        .route("/api/v1/user/stats", get(user_stats))
        .route(
            "/api/v1/user/preferences",
            get(get_user_preferences).put(update_user_preferences),
        )
        // Booking checkin
        .route("/api/v1/bookings/{id}/checkin", post(booking_checkin))
        // Admin: database reset
        .route("/api/v1/admin/reset", post(admin_reset))
        // Admin: auto-release settings
        .route(
            "/api/v1/admin/settings/auto-release",
            get(admin_get_auto_release).put(admin_update_auto_release),
        )
        // Admin: email settings
        .route(
            "/api/v1/admin/settings/email",
            get(admin_get_email_settings).put(admin_update_email_settings),
        )
        // Admin: privacy settings
        .route(
            "/api/v1/admin/privacy",
            get(admin_get_privacy).put(admin_update_privacy),
        )
        // Admin: update user
        .route("/api/v1/admin/users/{id}/update", put(admin_update_user))
        // Translation management
        .route("/api/v1/translations/overrides", get(list_overrides))
        .route(
            "/api/v1/translations/proposals",
            get(list_proposals).post(create_proposal),
        )
        .route(
            "/api/v1/translations/proposals/{id}",
            get(get_proposal),
        )
        .route(
            "/api/v1/translations/proposals/{id}/vote",
            post(vote_on_proposal),
        )
        .route(
            "/api/v1/translations/proposals/{id}/review",
            put(review_proposal),
        )
        // Payments (Stripe stub)
        .route("/api/v1/payments/create-intent", post(payments::create_payment_intent))
        .route("/api/v1/payments/confirm", post(payments::confirm_payment))
        .route("/api/v1/payments/{id}/status", get(payments::payment_status))
        .layer(Extension(payments::new_payment_store()))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Demo mode routes (no auth — by design for public demo, rate-limited POST endpoints)
    let demo_state = demo::new_demo_state();
    let demo_state_ret = demo_state.clone();
    let demo_limiter = rate_limiters.demo;
    let demo_limiter2 = demo_limiter.clone();
    let demo_vote_route = Router::new()
        .route("/api/v1/demo/vote", post(demo::demo_vote))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(demo_limiter.clone(), req, next)
        }));
    let demo_reset_route = Router::new()
        .route("/api/v1/demo/reset", post(demo::demo_reset))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(demo_limiter2.clone(), req, next)
        }));
    let demo_routes = Router::new()
        .route("/api/v1/demo/status", get(demo::demo_status))
        .route("/api/v1/demo/config", get(demo::demo_config))
        .merge(demo_vote_route)
        .merge(demo_reset_route)
        .layer(Extension(demo_state))
        .layer(Extension(state.clone()));

    // Clone handle for the closure
    let metrics_handle_clone = metrics_handle;

    // Build CORS allowed-origins list.
    // `PARKHUB_CORS_ORIGINS` can be set to a comma-separated list of allowed origins
    // (e.g. "https://parkhub.example.com,https://admin.example.com").
    // When unset or empty, only localhost/127.0.0.1 origins are permitted (dev default).
    let extra_origins: Vec<String> = std::env::var("PARKHUB_CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let x_request_id = HeaderName::from_static("x-request-id");

    let router = Router::new()
        .merge(public_routes)
        .merge(login_route)
        .merge(register_route)
        .merge(forgot_route)
        .merge(qr_route)
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
                            .is_some_and(|token| token == expected);
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
        // ── Outermost layers (applied last-in-first-out) ──────────────
        // Propagate x-request-id from response back to the client
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        // HTTP request metrics — captures method, path, status, duration for Prometheus
        .layer(axum::middleware::from_fn(http_metrics_middleware))
        // Request-ID tracing middleware — logs request_id in every span
        .layer(axum::middleware::from_fn(request_id_tracing_middleware))
        .layer(TraceLayer::new_for_http())
        // Response compression (gzip + brotli) — negotiated via Accept-Encoding
        .layer(CompressionLayer::new().gzip(true).br(true))
        // Global rate limit — 100 req/s with burst 200
        .layer(axum::middleware::from_fn(move |req, next| {
            crate::rate_limit::rate_limit_middleware(global_limiter.clone(), req, next)
        }))
        // Security headers applied to every response
        .layer(axum::middleware::from_fn(security_headers_middleware))
        // Restrict request body size to prevent DoS via large payloads
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        // CORS: same-origin by default; no wildcard.
        // Set PARKHUB_CORS_ORIGINS for production deployments.
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(
                    move |origin: &HeaderValue, _req_parts: &axum::http::request::Parts| {
                        let s = origin.to_str().unwrap_or("");
                        // Always allow localhost / 127.0.0.1 for development
                        if s.starts_with("http://localhost:")
                            || s.starts_with("https://localhost:")
                            || s.starts_with("http://127.0.0.1:")
                        {
                            return true;
                        }
                        // Allow origins from PARKHUB_CORS_ORIGINS env var
                        extra_origins.iter().any(|allowed| s == allowed)
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
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                    HeaderName::from_static("x-request-id"),
                ])
                .expose_headers([HeaderName::from_static("x-request-id")])
                .allow_credentials(false),
        )
        // Assign a unique x-request-id to every inbound request (UUID v4)
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        // 30-second request timeout — returns 408 on breach
        .layer(
            ServiceBuilder::new()
                .layer(axum::error_handling::HandleErrorLayer::new(
                    |_: tower::BoxError| async {
                        (
                            StatusCode::REQUEST_TIMEOUT,
                            Json(serde_json::json!({
                                "error": "REQUEST_TIMEOUT",
                                "message": "Request timed out"
                            })),
                        )
                    },
                ))
                .layer(tower::timeout::TimeoutLayer::new(Duration::from_secs(30))),
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
// REQUEST ID TRACING MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

/// Attaches the `x-request-id` header value (set by `SetRequestIdLayer`) to the
/// current tracing span so that every log line within this request includes the
/// request ID.  Also ensures the header is copied to the response.
async fn request_id_tracing_middleware(request: Request<Body>, next: Next) -> Response {
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
async fn http_metrics_middleware(request: Request<Body>, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let status = response.status().as_u16();
    let duration = start.elapsed();

    // Normalize path to avoid high-cardinality labels (strip UUIDs/IDs)
    let normalized = normalize_metric_path(&path);
    metrics::record_http_request(&method, &normalized, status, duration);

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
fn normalize_metric_path(path: &str) -> String {
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
// VEHICLES
// ═══════════════════════════════════════════════════════════════════════════════

#[utoipa::path(get, path = "/api/v1/vehicles", tag = "Vehicles",
    summary = "List user's vehicles",
    description = "Returns all vehicles registered by the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of vehicles"))
)]
pub async fn list_vehicles(
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

#[utoipa::path(post, path = "/api/v1/vehicles", tag = "Vehicles",
    summary = "Register a new vehicle",
    description = "Adds a vehicle to the authenticated user's account.",
    security(("bearer_auth" = [])),
    request_body = VehicleRequest,
    responses((status = 201, description = "Vehicle created"))
)]
pub async fn create_vehicle(
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
#[utoipa::path(delete, path = "/api/v1/vehicles/{id}", tag = "Vehicles",
    summary = "Delete a vehicle", description = "Removes a vehicle. Only the owner can delete.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
pub async fn delete_vehicle(
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
#[utoipa::path(put, path = "/api/v1/vehicles/{id}", tag = "Vehicles",
    summary = "Update a vehicle", description = "Updates vehicle details. Only the owner can update.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
pub async fn update_vehicle(
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
    if let Some(is_default) = req.get("is_default").and_then(serde_json::Value::as_bool) {
        vehicle.is_default = is_default;
    }

    if let Err(e) = state_guard.db.save_vehicle(&vehicle).await {
        tracing::error!("Failed to update vehicle: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update vehicle",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(vehicle)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// VEHICLE PHOTOS
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for uploading a vehicle photo as base64-encoded image data.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VehiclePhotoUpload {
    /// Base64-encoded image, optionally prefixed with a data URI scheme
    /// (e.g. `data:image/jpeg;base64,...`).
    photo: String,
}

/// Detect image format from decoded bytes via magic number.
/// Returns the MIME content-type string or `None` if unrecognised.
fn detect_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        Some("image/jpeg")
    } else if bytes.len() >= 4
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        Some("image/png")
    } else {
        None
    }
}

/// Strip an optional `data:<mime>;base64,` prefix and return the raw base64 payload.
fn strip_data_uri_prefix(input: &str) -> &str {
    input
        .find(";base64,")
        .map_or(input, |pos| &input[pos + 8..])
}

/// `POST /api/v1/vehicles/{id}/photo` — upload a vehicle photo (base64 JSON body).
///
/// Validates ownership, image magic bytes (JPEG / PNG), and a 2 MB size cap.
/// Stores the raw base64 payload in the DB settings table under key
/// `vehicle_photo_{vehicle_id}`.
#[utoipa::path(post, path = "/api/v1/vehicles/{id}/photo", tag = "Vehicles",
    summary = "Upload vehicle photo", description = "Uploads a base64-encoded vehicle photo (JPEG or PNG, max 2 MB).",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Photo uploaded"), (status = 400, description = "Invalid image"), (status = 404, description = "Not found"))
)]
pub async fn upload_vehicle_photo(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<VehiclePhotoUpload>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify vehicle exists and belongs to caller
    let vehicle = match state_guard.db.get_vehicle(&id).await {
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

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    let b64_payload = strip_data_uri_prefix(&req.photo);

    // Decode base64 to validate the image
    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64_payload) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "Invalid base64 data")),
        );
    };

    // Size check (2 MB max)
    if raw_bytes.len() > MAX_PHOTO_BYTES {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "PAYLOAD_TOO_LARGE",
                "Photo exceeds 2 MB limit",
            )),
        );
    }

    // Validate magic bytes
    if detect_image_mime(&raw_bytes).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Unsupported image format. Only JPEG and PNG are accepted.",
            )),
        );
    }

    // Store the full data URI (or raw base64) so we can reconstruct content-type on GET
    let key = format!("vehicle_photo_{id}");
    let value = req.photo.clone();

    if let Err(e) = state_guard.db.set_setting(&key, &value).await {
        tracing::error!("Failed to save vehicle photo: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save photo")),
        );
    }

    tracing::info!(vehicle_id = %id, bytes = raw_bytes.len(), "Vehicle photo uploaded");
    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// `GET /api/v1/vehicles/{id}/photo` — download a vehicle photo.
///
/// Returns the binary image with the correct `Content-Type` header.
/// If no photo is stored, returns 404.
#[utoipa::path(get, path = "/api/v1/vehicles/{id}/photo", tag = "Vehicles",
    summary = "Download vehicle photo", description = "Returns the stored vehicle photo as binary.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Vehicle UUID")),
    responses((status = 200, description = "Photo bytes"), (status = 404, description = "No photo"))
)]
pub async fn get_vehicle_photo(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> Response {
    let state_guard = state.read().await;

    // Verify vehicle exists and belongs to caller
    let Ok(Some(vehicle)) = state_guard.db.get_vehicle(&id).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("NOT_FOUND", "Vehicle not found")),
        )
            .into_response();
    };

    if vehicle.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("FORBIDDEN", "Access denied")),
        )
            .into_response();
    }

    let key = format!("vehicle_photo_{id}");
    let Ok(Some(stored)) = state_guard.db.get_setting(&key).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("NOT_FOUND", "No photo found")),
        )
            .into_response();
    };

    let b64_payload = strip_data_uri_prefix(&stored);

    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64_payload) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SERVER_ERROR",
                "Corrupt photo data",
            )),
        )
            .into_response();
    };

    let content_type = detect_image_mime(&raw_bytes).unwrap_or("application/octet-stream");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        raw_bytes,
    )
        .into_response()
}

// ═══════════════════════════════════════════════════════════════════════════════
// GERMAN LICENSE PLATE CITY CODES
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/vehicles/city-codes` — return a JSON map of the most common
/// German licence-plate area codes to their city/district names.
#[utoipa::path(get, path = "/api/v1/vehicles/city-codes", tag = "Vehicles",
    summary = "German license plate city codes",
    description = "Returns a map of German Kfz-Kennzeichen area codes to city names.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "City code map"))
)]
pub async fn vehicle_city_codes(
) -> Json<ApiResponse<std::collections::HashMap<&'static str, &'static str>>> {
    // Top ~75 German Kfz-Kennzeichen area codes → city/district names.
    // Built once per call (tiny cost) to avoid the serde_json::json! recursion limit.
    let codes: std::collections::HashMap<&str, &str> = [
        ("A", "Augsburg"),
        ("AA", "Aalen"),
        ("AB", "Aschaffenburg"),
        ("AC", "Aachen"),
        ("AK", "Altenkirchen"),
        ("B", "Berlin"),
        ("BA", "Bamberg"),
        ("BB", "Böblingen"),
        ("BC", "Biberach"),
        ("BN", "Bonn"),
        ("C", "Chemnitz"),
        ("CB", "Cottbus"),
        ("CE", "Celle"),
        ("CO", "Coburg"),
        ("CW", "Calw"),
        ("D", "Düsseldorf"),
        ("DA", "Darmstadt"),
        ("DD", "Dresden"),
        ("DE", "Dessau"),
        ("DO", "Dortmund"),
        ("DU", "Duisburg"),
        ("E", "Essen"),
        ("EM", "Emmendingen"),
        ("ER", "Erlangen"),
        ("ES", "Esslingen"),
        ("F", "Frankfurt"),
        ("FB", "Friedberg"),
        ("FN", "Friedrichshafen"),
        ("FR", "Freiburg"),
        ("FT", "Frankenthal"),
        ("G", "Gera"),
        ("GI", "Gießen"),
        ("GP", "Göppingen"),
        ("GÖ", "Göttingen"),
        ("H", "Hannover"),
        ("HA", "Hagen"),
        ("HB", "Bremen"),
        ("HD", "Heidelberg"),
        ("HH", "Hamburg"),
        ("HN", "Heilbronn"),
        ("K", "Köln"),
        ("KA", "Karlsruhe"),
        ("KI", "Kiel"),
        ("KN", "Konstanz"),
        ("KS", "Kassel"),
        ("L", "Leipzig"),
        ("LB", "Ludwigsburg"),
        ("LU", "Ludwigshafen"),
        ("M", "München"),
        ("MA", "Mannheim"),
        ("MD", "Magdeburg"),
        ("MH", "Mülheim"),
        ("MK", "Märkischer Kreis"),
        ("MS", "Münster"),
        ("N", "Nürnberg"),
        ("NE", "Neuss"),
        ("OB", "Oberhausen"),
        ("OF", "Offenbach"),
        ("OL", "Oldenburg"),
        ("OS", "Osnabrück"),
        ("P", "Potsdam"),
        ("PB", "Paderborn"),
        ("PF", "Pforzheim"),
        ("R", "Regensburg"),
        ("RE", "Recklinghausen"),
        ("RO", "Rosenheim"),
        ("RS", "Remscheid"),
        ("RT", "Reutlingen"),
        ("S", "Stuttgart"),
        ("SG", "Solingen"),
        ("SN", "Schwerin"),
        ("SO", "Soest"),
        ("ST", "Steinfurt"),
        ("UL", "Ulm"),
        ("UN", "Unna"),
        ("W", "Wuppertal"),
        ("WI", "Wiesbaden"),
        ("WÜ", "Würzburg"),
    ]
    .into_iter()
    .collect();
    Json(ApiResponse::success(codes))
}

// ═══════════════════════════════════════════════════════════════════════════════
// QR CODE GENERATION (EXTERNAL SERVICE)
// ═══════════════════════════════════════════════════════════════════════════════

/// QR code response with URLs for generating a QR code externally.
#[derive(Debug, Serialize)]
pub struct LotQrResponse {
    /// URL to an external QR code image service.
    /// NOTE: Uses api.qrserver.com. For privacy-sensitive deployments,
    /// operators should generate QR codes locally.
    qr_url: String,
    /// The lot's booking page URL that the QR code encodes.
    lot_url: String,
}

/// `GET /api/v1/lots/{id}/qr` — generate a QR code URL for a parking lot.
///
/// Returns a URL pointing to an external QR API (api.qrserver.com) that renders
/// a QR code linking to the lot's booking page.
#[utoipa::path(get, path = "/api/v1/lots/{id}/qr", tag = "Lots",
    summary = "Generate QR code URL for a lot",
    description = "Returns a URL to an external QR service encoding the lot's booking page.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Lot UUID")),
    responses((status = 200, description = "QR URLs"), (status = 404, description = "Lot not found"))
)]
pub async fn lot_qr_code(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<LotQrResponse>>) {
    let state_guard = state.read().await;

    // Verify lot exists
    match state_guard.db.get_parking_lot(&id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking lot not found")),
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

    // Derive base_url from admin setting or fall back to localhost
    let base_url = read_admin_setting(&state_guard.db, "base_url").await;
    let base_url = if base_url.is_empty() {
        format!("https://localhost:{}", state_guard.config.port)
    } else {
        base_url
    };

    let lot_url = format!("{base_url}/book?lot={id}");
    let encoded = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("data", &lot_url)
        .append_pair("size", "256x256")
        .finish();
    let qr_url = format!("https://api.qrserver.com/v1/create-qr-code/?{encoded}");

    (
        StatusCode::OK,
        Json(ApiResponse::success(LotQrResponse { qr_url, lot_url })),
    )
}


// ═══════════════════════════════════════════════════════════════════════════════
// TOKEN GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a cryptographically random access token (32 bytes, hex-encoded).
///
/// UUIDs v4 have a fixed structure that reduces effective entropy to ~122 bits.
/// Using a raw 256-bit random value is both simpler and more secure.
pub fn generate_access_token() -> String {
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
pub fn hash_password(
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
pub fn hash_password_simple(password: &str) -> anyhow::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// ═══════════════════════════════════════════════════════════════════════════════
// PUBLIC OCCUPANCY
// ═══════════════════════════════════════════════════════════════════════════════

/// Occupancy info for a single lot
#[derive(Debug, Serialize)]
pub struct LotOccupancy {
    lot_id: String,
    lot_name: String,
    total_slots: i32,
    occupied_slots: i32,
    available_slots: i32,
}

/// `GET /api/v1/public/occupancy` — public JSON occupancy data
#[utoipa::path(get, path = "/api/v1/public/occupancy", tag = "Public",
    summary = "Public lot occupancy",
    description = "Returns real-time occupancy. No auth required.",
    responses((status = 200, description = "Success"))
)]
pub async fn public_occupancy(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<Vec<LotOccupancy>>>) {
    let state_guard = state.read().await;

    let lots = match state_guard.db.list_parking_lots().await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to list lots for occupancy: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to get occupancy",
                )),
            );
        }
    };

    // Count active bookings per lot
    let now = Utc::now();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut occupancy = Vec::with_capacity(lots.len());
    for lot in &lots {
        let occupied = i32::try_from(
            bookings
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
                .count(),
        )
        .unwrap_or(i32::MAX);

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
#[utoipa::path(get, path = "/api/v1/public/display", tag = "Public",
    summary = "Public display HTML",
    description = "Returns minimal HTML for digital signage.",
    responses((status = 200, description = "Success"))
)]
pub async fn public_display(State(state): State<SharedState>) -> impl axum::response::IntoResponse {
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
        let occupied = i32::try_from(
            bookings
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
                .count(),
        )
        .unwrap_or(i32::MAX);

        let available = (lot.total_slots - occupied).max(0);
        let pct = if lot.total_slots > 0 {
            (f64::from(available) / f64::from(lot.total_slots)) * 100.0
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

        {
            let _ = write!(
                html,
                r#"<div class="lot">
  <div class="lot-name">{}</div>
  <div class="available {}">{}</div>
  <div class="label">of {} available</div>
</div>
"#,
                crate::utils::html_escape(&lot.name),
                color_class,
                available,
                lot.total_slots
            );
        }
    }

    html.push_str("</div>\n</body>\n</html>\n");

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
}

