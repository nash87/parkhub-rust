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
pub(super) const MAX_PHOTO_BYTES: usize = 2 * 1024 * 1024;

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

pub mod admin;
pub mod auth;
mod bookings;
pub mod branding;
pub mod credits;
pub mod export;
pub mod import;
pub mod favorites;
pub mod ical_import;
pub mod import;
pub mod lots;
pub mod payments;
pub mod push;
pub mod pwa;
pub mod qr;
pub mod recommendations;
pub mod setup;
mod social;
pub mod translations;
mod users;
pub mod vehicles;
pub mod webhooks;
pub mod ws;
pub mod zones;

// Re-import handler functions so the router can reference them unqualified.
use auth::{forgot_password, login, refresh_token, register, reset_password};
use credits::{
    admin_grant_credits, admin_list_credit_transactions, admin_refill_all_credits,
    admin_update_user_quota, get_user_credits,
};
use vehicles::{
    create_vehicle, delete_vehicle, get_vehicle_photo, list_vehicles, update_vehicle,
    upload_vehicle_photo, vehicle_city_codes,
};
use export::{admin_export_bookings_csv, admin_export_revenue_csv, admin_export_users_csv};
use import::import_users_csv;
use favorites::{add_favorite, list_favorites, remove_favorite};
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_pricing, get_lot_slots,
    list_lots, update_lot, update_lot_pricing, update_slot,
};
use recommendations::get_recommendations;
use translations::{
    create_proposal, get_proposal, list_overrides, list_proposals, review_proposal,
    vote_on_proposal,
};
use webhooks::{create_webhook, delete_webhook, list_webhooks, test_webhook, update_webhook};
use zones::{create_zone, delete_zone, list_zones, update_zone};

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

    // POST /api/v1/auth/refresh — 10 requests per minute per IP
    let refresh_limiter = rate_limiters.token_refresh.clone();
    let refresh_route = Router::new()
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(refresh_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/reset-password — 5 requests per 15 minutes per IP
    let reset_pw_limiter = rate_limiters.password_reset.clone();
    let reset_password_route = Router::new()
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(reset_pw_limiter.clone(), req, next)
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
        // PWA manifest and service worker (no auth)
        .route("/manifest.json", get(pwa::pwa_manifest))
        .route("/sw.js", get(pwa::service_worker))
        // Branding logo (public, cached)
        .route("/api/v1/branding/logo", get(branding::get_branding_logo))
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
        // Per-lot pricing
        .route(
            "/api/v1/lots/{id}/pricing",
            get(get_lot_pricing).put(update_lot_pricing),
        )
        // QR code for lot
        .route("/api/v1/lots/{id}/qr", get(lot_qr_code))
        // QR code for individual slot
        .route("/api/v1/lots/{lot_id}/slots/{slot_id}/qr", get(qr::slot_qr_code))
        // Zones (admin-only CRUD, nested under lots)
        .route(
            "/api/v1/lots/{lot_id}/zones",
            get(list_zones).post(create_zone),
        )
        .route(
            "/api/v1/lots/{lot_id}/zones/{zone_id}",
            put(update_zone).delete(delete_zone),
        )
        .route("/api/v1/bookings", get(list_bookings).post(create_booking))
        .route(
            "/api/v1/bookings/{id}",
            get(get_booking)
                .delete(cancel_booking)
                .patch(update_booking),
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
            "/api/v1/admin/credits/transactions",
            get(admin_list_credit_transactions),
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
        // Admin-only: CSV import
        .route(
            "/api/v1/admin/users/import",
            post(import_users_csv),
        )
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
        // Admin-only: CSV import
        .route(
            "/api/v1/admin/users/import",
            post(import::import_users_csv),
        )
        // Absences (user-scoped)
        .route("/api/v1/absences", get(list_absences).post(create_absence))
        .route(
            "/api/v1/absences/import",
            post(ical_import::import_absences_ical),
        )
        .route("/api/v1/absences/team", get(list_team_absences))
        .route(
            "/api/v1/absences/pattern",
            get(get_absence_pattern).post(save_absence_pattern),
        )
        .route(
            "/api/v1/absences/{id}",
            delete(delete_absence).put(update_absence),
        )
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
            delete(delete_recurring_booking).put(update_recurring_booking),
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
        // Admin: branding config
        .route(
            "/api/v1/admin/branding",
            get(branding::admin_get_branding).put(branding::admin_update_branding),
        )
        .route(
            "/api/v1/admin/branding/logo",
            post(branding::admin_upload_logo),
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
        .merge(refresh_route)
        .merge(reset_password_route)
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

    // Re-validate the user against the DB: reject disabled or deleted accounts
    // even when their session token is still technically valid. This prevents
    // suspended users from continuing to make requests until their token expires.
    match state_guard
        .db
        .get_user(&session.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.is_active => {}
        Ok(Some(_)) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "ACCOUNT_DISABLED",
                    "Account is disabled",
                )),
            ));
        }
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "UNAUTHORIZED",
                    "User not found",
                )),
            ));
        }
    }
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
// USERS
// ═══════════════════════════════════════════════════════════════════════════════

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "Users",
    summary = "Get current user profile",
    description = "Returns the authenticated user's profile.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "User profile"),
        (status = 404, description = "User not found")
    )
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn get_current_user(
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
            tracing::error!(error = %e, "Failed to fetch current user");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        }
    }
}

/// Request body for updating the current user's profile
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateCurrentUserRequest {
    name: Option<String>,
    phone: Option<String>,
    picture: Option<String>,
}

/// `PUT /api/v1/users/me` — update the authenticated user's own profile.
///
/// Allows users to update their display name, phone number, and profile
/// picture URL. Fields not included in the request body are left unchanged.
/// Returns the updated user record (without `password_hash`).
#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "Users",
    summary = "Update current user profile",
    description = "Updates the authenticated user's display name, phone, and/or profile picture.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Profile updated"),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "User not found")
    )
)]
pub async fn update_current_user(
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
/// Restricted to Admin and `SuperAdmin` roles. Regular users must use
/// `GET /api/v1/users/me` to access their own profile.
#[utoipa::path(get, path = "/api/v1/users/{id}", tag = "Admin",
    summary = "Get user by ID (admin)",
    description = "Retrieves any user's profile. Admin/SuperAdmin only.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "User found"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
pub async fn get_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<User>>) {
    let state = state.read().await;

    // Verify caller is an admin before exposing arbitrary user records.
    let Ok(Some(caller)) = state.db.get_user(&auth_user.user_id.to_string()).await else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
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

#[utoipa::path(get, path = "/api/v1/bookings", tag = "Bookings",
    summary = "List current user's bookings",
    description = "Returns all bookings for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of bookings"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn list_bookings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Booking>>> {
    let state = state.read().await;

    match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(bookings) => {
            tracing::debug!(count = bookings.len(), "Listed bookings");
            Json(ApiResponse::success(bookings))
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to list bookings");
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list bookings",
            ))
        }
    }
}

#[utoipa::path(post, path = "/api/v1/bookings", tag = "Bookings",
    summary = "Create a new booking",
    description = "Books a parking slot for the authenticated user.",
    security(("bearer_auth" = [])),
    request_body = CreateBookingRequest,
    responses((status = 201, description = "Booking created"), (status = 404, description = "Not found"), (status = 409, description = "Slot unavailable"))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, slot_id = %req.slot_id))]
pub async fn create_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    // ── Phase 1: reads under a read lock ──────────────────────────────────────
    // Collect all data needed to validate and price the booking.  A read lock
    // allows concurrent readers; we release it before any mutation.
    let (
        slot,
        vehicle,
        require_vehicle,
        plate_mode,
        duration_hours,
        min_hours,
        max_hours,
        max_per_day,
        same_day_count,
        credits_enabled,
        credits_per_booking,
        mut booking_user,
        lot_opt,
        org_name,
    ) = {
        let rg = state.read().await;

        // Check if slot exists and is available
        let slot = match rg.db.get_parking_slot(&req.slot_id.to_string()).await {
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
        let vehicle = match rg.db.get_vehicle(&req.vehicle_id.to_string()).await {
            Ok(Some(v)) => {
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

        // Admin settings
        let require_vehicle = read_admin_setting(&rg.db, "require_vehicle").await;
        let plate_mode = read_admin_setting(&rg.db, "license_plate_mode").await;
        let duration_hours = f64::from(req.duration_minutes) / 60.0;
        let min_hours: f64 = read_admin_setting(&rg.db, "min_booking_duration_hours")
            .await
            .parse()
            .unwrap_or(0.0);
        let max_hours: f64 = read_admin_setting(&rg.db, "max_booking_duration_hours")
            .await
            .parse()
            .unwrap_or(0.0);
        let max_per_day: i32 = read_admin_setting(&rg.db, "max_bookings_per_day")
            .await
            .parse()
            .unwrap_or(0);

        let same_day_count = if max_per_day > 0 {
            let user_bookings = rg
                .db
                .list_bookings_by_user(&auth_user.user_id.to_string())
                .await
                .unwrap_or_default();
            let booking_date = req.start_time.date_naive();
            user_bookings
                .iter()
                .filter(|b| {
                    b.start_time.date_naive() == booking_date
                        && b.status != BookingStatus::Cancelled
                })
                .count()
        } else {
            0
        };

        // Credits settings
        let credits_enabled = rg
            .db
            .get_setting("credits_enabled")
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
            == "true";
        let credits_per_booking: i32 = rg
            .db
            .get_setting("credits_per_booking")
            .await
            .ok()
            .flatten()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let booking_user = match rg.db.get_user(&auth_user.user_id.to_string()).await {
            Ok(Some(u)) => u,
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to load user")),
                );
            }
        };

        let lot_opt = rg
            .db
            .get_parking_lot(&req.lot_id.to_string())
            .await
            .ok()
            .flatten();

        let org_name = rg.config.organization_name.clone();

        (
            slot,
            vehicle,
            require_vehicle,
            plate_mode,
            duration_hours,
            min_hours,
            max_hours,
            max_per_day,
            same_day_count,
            credits_enabled,
            credits_per_booking,
            booking_user,
            lot_opt,
            org_name,
        )
    };
    // Read lock released here.

    // ── Stateless validation (no lock needed) ─────────────────────────────────

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

    if require_vehicle == "true" && req.vehicle_id == Uuid::nil() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VEHICLE_REQUIRED",
                "A vehicle is required for booking",
            )),
        );
    }

    if plate_mode == "required" && req.license_plate.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "LICENSE_PLATE_REQUIRED",
                "A license plate is required for booking",
            )),
        );
    }

    if min_hours > 0.0 && duration_hours < min_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_SHORT",
                format!("Minimum booking duration is {min_hours} hour(s)"),
            )),
        );
    }

    if max_hours > 0.0 && duration_hours > max_hours {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "DURATION_TOO_LONG",
                format!("Maximum booking duration is {max_hours} hour(s)"),
            )),
        );
    }

    if max_per_day > 0 && same_day_count >= usize::try_from(max_per_day).unwrap_or(0) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "MAX_BOOKINGS_REACHED",
                format!("Maximum of {max_per_day} booking(s) per day reached"),
            )),
        );
    }

    // ── End admin settings enforcement ──────────────────────────────────────

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

    // Calculate pricing (no lock needed)
    let end_time = req.start_time + TimeDelta::minutes(i64::from(req.duration_minutes));

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map_or(2.0, |r| r.price);
    let daily_max = lot_opt.as_ref().and_then(|lot| lot.pricing.daily_max);
    let lot_currency = lot_opt
        .as_ref()
        .map(|lot| lot.pricing.currency.clone())
        .unwrap_or_else(|| "EUR".to_string());

    // Cap at daily_max if configured (e.g. all-day price ceiling)
    let raw_price = (f64::from(req.duration_minutes) / 60.0) * hourly_rate;
    let base_price = if let Some(cap) = daily_max {
        raw_price.min(cap)
    } else {
        raw_price
    };
    let tax = base_price * VAT_RATE;
    let total = base_price + tax;

    let floor_name = lot_opt.as_ref().map_or_else(
        || "Level 1".to_string(),
        |lot| {
            lot.floors
                .iter()
                .find(|f| f.id == slot.floor_id)
                .map_or_else(|| "Level 1".to_string(), |f| f.name.clone())
        },
    );

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
            currency: lot_currency,
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

    // ── Phase 2: mutations under a write lock ──────────────────────────────────
    // Re-check slot availability and commit all mutations atomically.
    // The write lock serialises concurrent booking attempts for the same slot,
    // preventing double-booking between the availability check and the insert.
    let user_info_opt = {
        let state_guard = state.write().await;

        // Re-check slot availability now that we hold the write lock.
        match state_guard.db.get_parking_slot(&req.slot_id.to_string()).await {
            Ok(Some(s)) if s.status != SlotStatus::Available => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::error(
                        "SLOT_UNAVAILABLE",
                        "This slot is not available",
                    )),
                );
            }
            Err(e) => {
                tracing::error!("Database error on slot re-check: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
                );
            }
            _ => {}
        }

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

        // Update slot status atomically within the write-lock scope.
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

        let audit_entry = if let Some(ref u) = user_info_opt {
            crate::audit::events::booking_created(auth_user.user_id, &u.username, booking.id)
        } else {
            crate::audit::events::booking_created(auth_user.user_id, "", booking.id)
        };
        audit_entry.persist(&state_guard.db).await;

        // Write lock released at end of this block.
        user_info_opt
    };

    // Dispatch webhook event (non-blocking)
    {
        let state_clone = state.clone();
        let booking_json = serde_json::json!({
            "booking_id": booking.id,
            "user_id": auth_user.user_id,
            "lot_id": booking.lot_id,
            "slot_number": booking.slot_number,
            "start_time": booking.start_time,
            "end_time": booking.end_time,
        });
        tokio::spawn(async move {
            webhooks::dispatch_webhook_event(&state_clone, "booking.created", booking_json).await;
        });
    }
    metrics::record_booking_event("created");

    // Send booking confirmation email (non-blocking, fire-and-forget).
    if let Some(u) = user_info_opt {
        let booking_id_str = booking.id.to_string();
        let floor_name = booking.floor_name.clone();
        let slot_number = booking.slot_number;
        let start_time_str = booking.start_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let end_time_str = booking.end_time.format("%Y-%m-%d %H:%M UTC").to_string();
        let user_email = u.email.clone();
        let user_name = u.name;
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

#[utoipa::path(get, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Get booking by ID",
    description = "Returns a single booking. Only the owner can access it.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Booking found"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id, booking_id = %id))]
pub async fn get_booking(
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

#[utoipa::path(delete, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Cancel a booking",
    description = "Cancels an active booking and releases the slot.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Cancelled"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id, booking_id = %id))]
pub async fn cancel_booking(
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

    // Fetch user for audit log + cancellation email
    let user = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
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

    // Send cancellation confirmation email (async, best-effort)
    if let Some(ref user) = user {
        let user_email = user.email.clone();
        let user_name = user.name.clone();
        let booking_id_str = booking.id.to_string();
        let org_name = state_guard.config.organization_name.clone();
        let start_time = booking.start_time.format("%Y-%m-%d %H:%M").to_string();
        let end_time = booking.end_time.format("%Y-%m-%d %H:%M").to_string();
        let floor = booking.floor_name.clone();
        let slot = booking.slot_number;
        tokio::spawn(async move {
            let email_html = email::build_booking_cancellation_email(
                &user_name,
                &booking_id_str,
                &floor,
                slot,
                &start_time,
                &end_time,
                &org_name,
            );
            if let Err(e) =
                email::send_email(&user_email, "Booking Cancelled — ParkHub", &email_html).await
            {
                tracing::warn!("Failed to send cancellation email: {}", e);
            }
        });
    }

    // Notify the first waitlist member that a slot is now available (async, best-effort)
    {
        let state_clone = state.clone();
        let lot_id_str = booking.lot_id.to_string();
        let org_name_wl = state_guard.config.organization_name.clone();
        tokio::spawn(async move {
            let state_r = state_clone.read().await;
            let lot_name = state_r
                .db
                .get_parking_lot(&lot_id_str)
                .await
                .ok()
                .flatten()
                .map(|l| l.name)
                .unwrap_or_else(|| lot_id_str.clone());

            let waitlist = state_r
                .db
                .list_waitlist_by_lot(&lot_id_str)
                .await
                .unwrap_or_default();

            // Notify the earliest-queued user who has not yet been notified
            if let Some(entry) = waitlist.iter().find(|e| e.notified_at.is_none()) {
                if let Ok(Some(wl_user)) = state_r.db.get_user(&entry.user_id.to_string()).await {
                    let email_html = email::build_waitlist_slot_available_email(
                        &wl_user.name,
                        &lot_name,
                        &org_name_wl,
                    );
                    let subject = format!("Parking slot available at {lot_name} — ParkHub");
                    if let Err(e) =
                        email::send_email(&wl_user.email, &subject, &email_html).await
                    {
                        tracing::warn!("Failed to send waitlist notification: {}", e);
                    } else {
                        // Mark the entry as notified
                        let mut updated = entry.clone();
                        updated.notified_at = Some(Utc::now());
                        if let Err(e) = state_r.db.save_waitlist_entry(&updated).await {
                            tracing::warn!("Failed to update waitlist notified_at: {}", e);
                        }
                        tracing::info!(
                            user_id = %wl_user.id,
                            lot_id = %lot_id_str,
                            "Waitlist slot-available notification sent"
                        );
                    }
                }
            }
        });
    }

    // Dispatch webhook event
    {
        let state_clone = state.clone();
        let payload = serde_json::json!({
            "booking_id": id,
            "user_id": auth_user.user_id,
            "action": "cancelled",
        });
        tokio::spawn(async move {
            webhooks::dispatch_webhook_event(&state_clone, "booking.cancelled", payload).await;
        });
    }
    metrics::record_booking_event("cancelled");

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
#[utoipa::path(get, path = "/api/v1/bookings/{id}/invoice", tag = "Bookings",
    summary = "Download booking invoice",
    description = "Generates a text invoice for a booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn get_booking_invoice(
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
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            "Access denied".to_string(),
        );
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
// DASHBOARD CHARTS (ADMIN)
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
struct BookingsByDay {
    date: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct BookingsByLot {
    lot_name: String,
    count: usize,
}

#[derive(Debug, Serialize)]
struct OccupancyByHour {
    hour: u32,
    avg_occupancy: f64,
}

#[derive(Debug, Serialize)]
struct TopUser {
    username: String,
    booking_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DashboardCharts {
    bookings_by_day: Vec<BookingsByDay>,
    bookings_by_lot: Vec<BookingsByLot>,
    occupancy_by_hour: Vec<OccupancyByHour>,
    top_users: Vec<TopUser>,
}

/// `GET /api/v1/admin/dashboard/charts` — aggregated chart data for the admin
/// dashboard.  Returns bookings-by-day (last 30 days), bookings-by-lot,
/// average occupancy by hour-of-day, and top-10 users by booking count.
#[utoipa::path(get, path = "/api/v1/admin/dashboard/charts", tag = "Admin",
    summary = "Admin dashboard chart data",
    description = "Returns aggregated chart data for the admin dashboard.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Chart data"), (status = 403, description = "Forbidden"))
)]
pub async fn admin_dashboard_charts(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<DashboardCharts>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();
    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let users = state_guard.db.list_users().await.unwrap_or_default();
    let now = Utc::now();
    let cutoff = now - TimeDelta::days(30);

    // ── bookings_by_day (last 30 days) ──────────────────────────────────────
    let mut by_day: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    // Pre-fill all 30 days so the chart has continuous x-axis
    for d in 0..30 {
        let date = (now - TimeDelta::days(d)).format("%Y-%m-%d").to_string();
        by_day.entry(date).or_insert(0);
    }
    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *by_day.entry(date).or_insert(0) += 1;
        }
    }
    let bookings_by_day: Vec<BookingsByDay> = by_day
        .into_iter()
        .map(|(date, count)| BookingsByDay { date, count })
        .collect();

    // ── bookings_by_lot ─────────────────────────────────────────────────────
    let lot_name_map: std::collections::HashMap<Uuid, String> =
        lots.iter().map(|l| (l.id, l.name.clone())).collect();
    let mut by_lot: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in &bookings {
        let name = lot_name_map
            .get(&b.lot_id)
            .cloned()
            .unwrap_or_else(|| b.lot_id.to_string());
        *by_lot.entry(name).or_insert(0) += 1;
    }
    let mut bookings_by_lot: Vec<BookingsByLot> = by_lot
        .into_iter()
        .map(|(lot_name, count)| BookingsByLot { lot_name, count })
        .collect();
    bookings_by_lot.sort_by(|a, b| b.count.cmp(&a.count));

    // ── occupancy_by_hour (average across all lots) ─────────────────────────
    // For each hour of the day, count how many bookings are active during that
    // hour within the last 30 days, then divide by number of days with data.
    let total_slots: i32 = lots.iter().map(|l| l.total_slots).sum();
    let mut hour_totals = [0usize; 24];
    let mut hour_days = [0usize; 24];

    // Count distinct days per hour that had at least one booking
    let mut hour_day_set: [std::collections::HashSet<String>; 24] =
        std::array::from_fn(|_| std::collections::HashSet::new());

    for b in &bookings {
        if b.start_time >= cutoff || b.end_time >= cutoff {
            // Walk through each hour the booking spans
            let mut t = b.start_time;
            while t < b.end_time && t < now {
                let h = t.hour() as usize;
                if h < 24 {
                    hour_totals[h] += 1;
                    hour_day_set[h].insert(t.format("%Y-%m-%d").to_string());
                }
                t += TimeDelta::hours(1);
            }
        }
    }

    for (h, day_set) in hour_day_set.iter().enumerate() {
        hour_days[h] = day_set.len().max(1);
    }

    let occupancy_by_hour: Vec<OccupancyByHour> = (0..24)
        .map(|h| {
            #[allow(clippy::cast_precision_loss)]
            let avg_count = hour_totals[h] as f64 / hour_days[h] as f64;
            let avg_occ = if total_slots > 0 {
                (avg_count / f64::from(total_slots)).min(1.0)
            } else {
                0.0
            };
            OccupancyByHour {
                hour: u32::try_from(h).unwrap_or(0),
                avg_occupancy: (avg_occ * 100.0).round() / 100.0,
            }
        })
        .collect();

    // ── top_users (top 10 by booking count) ─────────────────────────────────
    let user_name_map: std::collections::HashMap<Uuid, String> =
        users.iter().map(|u| (u.id, u.username.clone())).collect();
    let mut by_user: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for b in &bookings {
        let name = user_name_map
            .get(&b.user_id)
            .cloned()
            .unwrap_or_else(|| b.user_id.to_string());
        *by_user.entry(name).or_insert(0) += 1;
    }
    let mut top_users: Vec<TopUser> = by_user
        .into_iter()
        .map(|(username, booking_count)| TopUser {
            username,
            booking_count,
        })
        .collect();
    top_users.sort_by(|a, b| b.booking_count.cmp(&a.booking_count));
    top_users.truncate(10);

    (
        StatusCode::OK,
        Json(ApiResponse::success(DashboardCharts {
            bookings_by_day,
            bookings_by_lot,
            occupancy_by_hour,
            top_users,
        })),
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

/// OWASP-recommended Argon2id parameters (2024).
///
/// - Memory:      65 536 KiB  (64 MiB) — OWASP minimum for interactive logins
/// - Iterations:  3           — balances security and latency on modern hardware
/// - Parallelism: 4           — matches typical server core count
///
/// These are set explicitly rather than relying on crate defaults so that
/// future crate upgrades cannot silently alter the tuning (issue #56).
fn argon2_params() -> argon2::Params {
    argon2::Params::new(65_536, 3, 4, None)
        .expect("OWASP Argon2 params are statically valid")
}

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
        Algorithm, Argon2, Version,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params());
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
        Algorithm, Argon2, Version,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params())
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Algorithm, Argon2, Version,
    };

    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };

    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params())
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
#[utoipa::path(get, path = "/api/v1/legal/impressum", tag = "Public",
    summary = "Get Impressum (public)", description = "Returns DDG paragraph 5 Impressum data. No auth required.",
    responses((status = 200, description = "Impressum fields"))
)]
pub async fn get_impressum(State(state): State<SharedState>) -> Json<serde_json::Value> {
    let mut data = serde_json::json!({});
    {
        let state = state.read().await;
        for field in IMPRESSUM_FIELDS {
            let key = format!("impressum_{field}");
            let value = state
                .db
                .get_setting(&key)
                .await
                .unwrap_or(None)
                .unwrap_or_default();
            data[field] = serde_json::Value::String(value);
        }
    }

    Json(data)
}

/// Admin: read Impressum settings (admin-only, protected).
///
/// Although the public endpoint exposes the same data, this route is kept
/// separate so admins can fetch the current values before editing them via PUT.
/// It is deliberately restricted to Admin/SuperAdmin.
#[utoipa::path(get, path = "/api/v1/admin/impressum", tag = "Admin",
    summary = "Get Impressum settings (admin)", description = "Returns current Impressum fields for editing. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Impressum fields"), (status = 403, description = "Forbidden"))
)]
pub async fn get_impressum_admin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<serde_json::Value>) {
    let state_guard = state.read().await;

    // Verify admin role.
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    };

    if caller.role != UserRole::Admin && caller.role != UserRole::SuperAdmin {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "FORBIDDEN", "message": "Admin access required"})),
        );
    }

    let mut data = serde_json::json!({});
    for field in IMPRESSUM_FIELDS {
        let key = format!("impressum_{field}");
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
#[utoipa::path(put, path = "/api/v1/admin/impressum", tag = "Admin",
    summary = "Update Impressum (admin)", description = "Saves DDG paragraph 5 Impressum fields. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Saved"), (status = 403, description = "Forbidden"))
)]
pub async fn update_impressum(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    // Verify admin role
    let user_id_str = auth_user.user_id.to_string();
    let state_guard = state.read().await;
    let Ok(Some(user)) = state_guard.db.get_user(&user_id_str).await else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin required")),
        );
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
            let key = format!("impressum_{field}");
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
#[utoipa::path(get, path = "/api/v1/users/me/export", tag = "Users",
    summary = "GDPR data export (Art. 15)", description = "Exports all personal data as JSON download.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "JSON data export"))
)]
pub async fn gdpr_export_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state = state.read().await;
    let user_id = auth_user.user_id.to_string();

    let Ok(Some(user)) = state.db.get_user(&user_id).await else {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&ApiResponse::<()>::error("NOT_FOUND", "User not found"))
                .unwrap_or_default(),
        );
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

    let absences = state
        .db
        .list_absences_by_user(&user_id)
        .await
        .unwrap_or_default();
    let credit_transactions = state
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default();
    let notifications = state
        .db
        .list_notifications_by_user(&user_id)
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
        "absences": absences,
        "credit_transactions": credit_transactions,
        "notifications": notifications,
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
#[utoipa::path(delete, path = "/api/v1/users/me/delete", tag = "Users",
    summary = "GDPR account deletion (Art. 17)", description = "Anonymizes user PII while preserving booking records.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Account anonymized"), (status = 404, description = "Not found"))
)]
pub async fn gdpr_delete_account(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRoleRequest {
    role: String,
}

/// Request body for updating a user's status
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserStatusRequest {
    status: String,
}

use admin::AdminUserResponse;

/// `GET /api/v1/admin/users` — list all users (admin only)
#[utoipa::path(get, path = "/api/v1/admin/users", tag = "Admin",
    summary = "List all users (admin)", description = "Returns all registered users. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "User list"), (status = 403, description = "Forbidden"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_list_users(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AdminUserResponse>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    match state_guard.db.list_users().await {
        Ok(users) => {
            tracing::debug!(count = users.len(), "Admin listed users");
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
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/role", tag = "Admin",
    summary = "Update user role (admin)", description = "Changes a user's role. Prevents privilege escalation.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Role updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, target_user_id = %id, new_role = %req.role))]
pub async fn admin_update_user_role(
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
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
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
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/status", tag = "Admin",
    summary = "Enable or disable a user (admin)", description = "Sets a user's active/inactive status. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, target_user_id = %id))]
pub async fn admin_update_user_status(
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

    let event_type = if user.is_active {
        AuditEventType::UserActivated
    } else {
        AuditEventType::UserDeactivated
    };
    let audit = AuditEntry::new(event_type)
        .user(auth_user.user_id, "admin")
        .resource("user", &id)
        .details(serde_json::json!({ "new_status": req.status }))
        .log();
    audit.persist(&state_guard.db).await;

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
#[utoipa::path(delete, path = "/api/v1/admin/users/{id}", tag = "Admin",
    summary = "Delete user (admin)", description = "Anonymizes user data per GDPR. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id, target_user_id = %id))]
pub async fn admin_delete_user(
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
pub struct AdminBookingResponse {
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
#[utoipa::path(get, path = "/api/v1/admin/bookings", tag = "Admin",
    summary = "List all bookings (admin)", description = "Returns all bookings with enriched details. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All bookings"), (status = 403, description = "Forbidden"))
)]
pub async fn admin_list_bookings(
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

    // Batch-load all users and lots upfront to avoid N+1 queries
    let all_users = state_guard.db.list_users().await.unwrap_or_default();
    let user_map: std::collections::HashMap<String, _> = all_users
        .into_iter()
        .map(|u| (u.id.to_string(), u))
        .collect();

    let all_lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let lot_map: std::collections::HashMap<String, _> = all_lots
        .into_iter()
        .map(|l| (l.id.to_string(), l))
        .collect();

    let mut response = Vec::with_capacity(bookings.len());
    for booking in bookings {
        let (user_name, user_email) = match user_map.get(&booking.user_id.to_string()) {
            Some(u) => (u.name.clone(), u.email.clone()),
            None => (booking.user_id.to_string(), String::new()),
        };

        let lot_name = match lot_map.get(&booking.lot_id.to_string()) {
            Some(l) => l.name.clone(),
            None => booking.lot_id.to_string(),
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
    ("use_case", "company"),
    ("self_registration", "true"),
    ("license_plate_mode", "optional"),
    ("display_name_format", "first_name"),
    ("max_bookings_per_day", "0"),
    ("allow_guest_bookings", "false"),
    ("auto_release_enabled", "false"),
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

/// Use-case theme definitions — maps `use_case` key to display config
fn use_case_theme(key: &str) -> serde_json::Value {
    match key {
        "company" => serde_json::json!({
            "key": "company",
            "name": "Company Parking",
            "description": "Employee parking for offices and campuses",
            "icon": "buildings",
            "primary_color": "#0d9488",
            "accent_color": "#0ea5e9",
            "terminology": {
                "user": "Employee", "users": "Employees",
                "lot": "Parking Area", "slot": "Spot",
                "booking": "Reservation", "department": "Department"
            },
            "features_emphasis": ["team_calendar", "absence_tracking", "departments", "credits"]
        }),
        "residential" => serde_json::json!({
            "key": "residential",
            "name": "Residential Parking",
            "description": "Parking for apartment buildings and housing complexes",
            "icon": "house-line",
            "primary_color": "#059669",
            "accent_color": "#84cc16",
            "terminology": {
                "user": "Resident", "users": "Residents",
                "lot": "Parking Area", "slot": "Space",
                "booking": "Reservation", "department": "Unit"
            },
            "features_emphasis": ["guest_parking", "long_term_bookings", "public_display"]
        }),
        "shared" => serde_json::json!({
            "key": "shared",
            "name": "Shared Parking",
            "description": "Community or co-working parking spaces",
            "icon": "users-three",
            "primary_color": "#7c3aed",
            "accent_color": "#06b6d4",
            "terminology": {
                "user": "Member", "users": "Members",
                "lot": "Parking Zone", "slot": "Spot",
                "booking": "Booking", "department": "Group"
            },
            "features_emphasis": ["quick_book", "waitlist", "public_display", "qr_codes"]
        }),
        "rental" => serde_json::json!({
            "key": "rental",
            "name": "Rental / Commercial",
            "description": "Paid parking for customers and tenants",
            "icon": "currency-circle-dollar",
            "primary_color": "#2563eb",
            "accent_color": "#f59e0b",
            "terminology": {
                "user": "Customer", "users": "Customers",
                "lot": "Parking Facility", "slot": "Bay",
                "booking": "Rental", "department": "Account"
            },
            "features_emphasis": ["invoicing", "pricing", "revenue_reports", "guest_bookings"]
        }),
        _ => serde_json::json!({
            "key": "personal",
            "name": "Personal / Private",
            "description": "Private parking for family and friends",
            "icon": "car-simple",
            "primary_color": "#e11d48",
            "accent_color": "#f97316",
            "terminology": {
                "user": "Person", "users": "People",
                "lot": "Driveway", "slot": "Spot",
                "booking": "Booking", "department": "Group"
            },
            "features_emphasis": ["simple_booking", "guest_parking"]
        }),
    }
}

/// `GET /api/v1/admin/settings/use-case` — return current use-case with theme config
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/use-case",
    tag = "Admin",
    summary = "Get use-case configuration",
    description = "Return current use-case with theme config. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_use_case(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }
    let current = read_admin_setting(&state_guard.db, "use_case").await;
    let theme = use_case_theme(&current);
    let all_options: Vec<serde_json::Value> =
        ["company", "residential", "shared", "rental", "personal"]
            .iter()
            .map(|k| use_case_theme(k))
            .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "current": theme,
            "available": all_options,
        }))),
    )
}

/// `GET /api/v1/admin/settings` — return all settings (merged defaults + stored values)
#[utoipa::path(get, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Get system settings (admin)", description = "Returns all system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Settings"), (status = 403, description = "Forbidden"))
)]
pub async fn admin_get_settings(
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
            if !["company", "residential", "shared", "rental", "personal"].contains(&value) {
                return Err("use_case must be company, residential, shared, rental, or personal");
            }
        }
        "self_registration"
        | "allow_guest_bookings"
        | "require_vehicle"
        | "waitlist_enabled"
        | "credits_enabled"
        | "auto_release_enabled" => {
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
#[utoipa::path(put, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Update system settings (admin)", description = "Saves system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Saved"), (status = 403, description = "Forbidden"))
)]
pub async fn admin_update_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(payload): Json<serde_json::Value>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let Some(obj) = payload.as_object() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Request body must be a JSON object of key-value pairs",
            )),
        );
    };

    let allowed_keys: Vec<&str> = ADMIN_SETTINGS.iter().map(|(k, _)| *k).collect();
    let mut updated = serde_json::Map::new();

    for (key, val) in obj {
        if !allowed_keys.contains(&key.as_str()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_KEY",
                    format!("Unknown setting: {key}"),
                )),
            );
        }

        let value_str = val
            .as_str()
            .map_or_else(|| val.to_string().trim_matches('"').to_string(), String::from);

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
        Ok(Some(json_str)) => serde_json::from_str::<Vec<String>>(&json_str).unwrap_or_else(|_| {
            DEFAULT_FEATURES
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        }),
        _ => DEFAULT_FEATURES
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
    }
}

/// `GET /api/v1/features` — public endpoint returning enabled features
#[utoipa::path(get, path = "/api/v1/features", tag = "Public",
    summary = "Get enabled feature flags",
    description = "Returns enabled and available features. No auth required.",
    responses((status = 200, description = "Success"))
)]
pub async fn get_features(
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

/// `GET /api/v1/theme` — public: return current use-case theme (no auth required)
#[utoipa::path(get, path = "/api/v1/theme", tag = "Public",
    summary = "Get current theme",
    description = "Returns theme and company name. No auth required.",
    responses((status = 200, description = "Success"))
)]
pub async fn get_public_theme(
    State(state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let use_case = read_admin_setting(&state_guard.db, "use_case").await;
    let company = read_admin_setting(&state_guard.db, "company_name").await;
    let theme = use_case_theme(&use_case);

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "use_case": theme,
            "company_name": company,
        }))),
    )
}

/// `GET /api/v1/admin/features` — admin: get features with full metadata
#[utoipa::path(get, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Get feature flags (admin)",
    description = "Returns feature modules with status. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_get_features(
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateFeaturesRequest {
    enabled: Vec<String>,
}

/// `PUT /api/v1/admin/features` — admin: update enabled features
#[utoipa::path(put, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Update feature flags (admin)",
    description = "Sets enabled feature modules. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_update_features(
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
pub struct AbsenceQuery {
    #[serde(rename = "type")]
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/absences` — list current user's absences, optionally filtered by type
#[utoipa::path(get, path = "/api/v1/absences", tag = "Absences",
    summary = "List user absences",
    description = "Returns absences for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_absences(
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAbsenceRequest {
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
#[utoipa::path(post, path = "/api/v1/absences", tag = "Absences",
    summary = "Create an absence",
    description = "Records a new absence for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn create_absence(
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
#[utoipa::path(delete, path = "/api/v1/absences/{id}", tag = "Absences",
    summary = "Delete an absence",
    description = "Removes an absence owned by the user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_absence(
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
#[utoipa::path(
    get,
    path = "/api/v1/absences/team",
    tag = "Absences",
    summary = "List team absences",
    description = "List all team member absences visible to the current user.",
    security(("bearer_auth" = []))
)]
pub async fn list_team_absences(
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
#[utoipa::path(
    get,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Get absence pattern",
    description = "Get the current user's recurring absence pattern.",
    security(("bearer_auth" = []))
)]
pub async fn get_absence_pattern(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Option<AbsencePattern>>>) {
    let state_guard = state.read().await;
    let key = format!("absence_pattern:{}", auth_user.user_id);
    match state_guard.db.get_setting(&key).await {
        Ok(val) => {
            let pattern = val.and_then(|json_str| serde_json::from_str::<AbsencePattern>(&json_str).ok());
            (StatusCode::OK, Json(ApiResponse::success(pattern)))
        }
        Err(e) => {
            tracing::error!("Failed to get absence pattern: {e}");
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
#[utoipa::path(
    post,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Save absence pattern",
    description = "Save or update the current user's recurring absence pattern (e.g. homeoffice every Monday).",
    security(("bearer_auth" = []))
)]
pub async fn save_absence_pattern(
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
pub struct TeamMemberStatus {
    user_id: Uuid,
    name: String,
    username: String,
    status: String,
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/team/today` — return all users with their status today
#[utoipa::path(get, path = "/api/v1/team/today", tag = "Team",
    summary = "Team status today",
    description = "Returns all users with their status for today.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn team_today(
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
                AbsenceType::Training | AbsenceType::Other => "absent",
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
#[utoipa::path(get, path = "/api/v1/announcements/active", tag = "Public",
    summary = "Get active announcements",
    description = "Returns active non-expired announcements. No auth required.",
    responses((status = 200, description = "Success"))
)]
pub async fn get_active_announcements(
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
                        && a.expires_at.is_none_or(|exp| exp > now)
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
#[utoipa::path(get, path = "/api/v1/admin/announcements", tag = "Admin",
    summary = "List all announcements (admin)",
    description = "Returns all announcements. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_list_announcements(
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAnnouncementRequest {
    title: String,
    message: String,
    severity: AnnouncementSeverity,
    active: Option<bool>,
    expires_at: Option<DateTime<Utc>>,
}

/// `POST /api/v1/admin/announcements` — admin: create announcement
#[utoipa::path(
    post,
    path = "/api/v1/admin/announcements",
    tag = "Admin",
    summary = "Create announcement",
    description = "Create a new system announcement. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_create_announcement(
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
        Ok(()) => {
            let audit = AuditEntry::new(AuditEventType::ConfigChanged)
                .user(auth_user.user_id, "admin")
                .resource("announcement", &announcement.id.to_string())
                .details(serde_json::json!({ "action": "create", "title": &announcement.title }))
                .log();
            audit.persist(&state_guard.db).await;
            (
                StatusCode::CREATED,
                Json(ApiResponse::success(announcement)),
            )
        }
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

/// Represents a field that can be absent, explicitly null, or a value.
/// This avoids `Option<Option<T>>` which clippy flags.
#[derive(Default)]
pub enum NullableField<T> {
    /// Field was not present in the request
    #[default]
    Absent,
    /// Field was explicitly set to null
    Null,
    /// Field was set to a value
    Value(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NullableField<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer)
            .map(|opt| opt.map_or(Self::Null, Self::Value))
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateAnnouncementRequest {
    title: Option<String>,
    message: Option<String>,
    severity: Option<AnnouncementSeverity>,
    active: Option<bool>,
    #[schema(value_type = Option<String>)]
    #[serde(default)]
    expires_at: NullableField<DateTime<Utc>>,
}

/// `PUT /api/v1/admin/announcements/{id}` — admin: update announcement
#[utoipa::path(
    put,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Update announcement",
    description = "Update an existing announcement by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_announcement(
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

    let Some(mut announcement) = announcements.into_iter().find(|a| a.id.to_string() == id)
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Announcement not found")),
        );
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
    match req.expires_at {
        NullableField::Value(v) => announcement.expires_at = Some(v),
        NullableField::Null => announcement.expires_at = None,
        NullableField::Absent => {}
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
#[utoipa::path(
    delete,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Delete announcement",
    description = "Delete an announcement by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_delete_announcement(
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
#[utoipa::path(get, path = "/api/v1/notifications", tag = "Notifications",
    summary = "List user notifications",
    description = "Returns recent notifications for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_notifications(
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
#[utoipa::path(put, path = "/api/v1/notifications/{id}/read", tag = "Notifications",
    summary = "Mark notification as read",
    description = "Marks a notification as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn mark_notification_read(
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
#[utoipa::path(post, path = "/api/v1/notifications/read-all", tag = "Notifications",
    summary = "Mark all notifications as read",
    description = "Marks all notifications as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn mark_all_notifications_read(
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
        if !notif.read
            && matches!(
                state_guard
                    .db
                    .mark_notification_read(&notif.id.to_string())
                    .await,
                Ok(true)
            )
        {
            count += 1;
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(count)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// WAITLIST
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/waitlist` — list current user's waitlist entries
#[utoipa::path(get, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "List waitlist entries",
    description = "Returns waitlist entries for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_waitlist(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct JoinWaitlistRequest {
    lot_id: Uuid,
}

/// `POST /api/v1/waitlist` — join waitlist for a lot
#[utoipa::path(post, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "Join waitlist",
    description = "Adds the user to a lot waitlist.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn join_waitlist(
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
#[utoipa::path(delete, path = "/api/v1/waitlist/{id}", tag = "Waitlist",
    summary = "Leave waitlist",
    description = "Removes the user from a waitlist entry.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn leave_waitlist(
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
#[utoipa::path(
    get,
    path = "/api/v1/swap-requests",
    tag = "Bookings",
    summary = "List swap requests",
    description = "List the current user's swap requests (as requester or target).",
    security(("bearer_auth" = []))
)]
pub async fn list_swap_requests(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSwapRequestBody {
    target_booking_id: Uuid,
    message: Option<String>,
}

/// `POST /api/v1/bookings/{id}/swap-request` — create a swap request
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/swap-request",
    tag = "Bookings",
    summary = "Create swap request",
    description = "Create a parking slot swap request for a booking.",
    security(("bearer_auth" = []))
)]
pub async fn create_swap_request(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSwapRequestBody {
    action: String,
}

/// `PUT /api/v1/swap-requests/{id}` — accept or decline a swap request
#[utoipa::path(
    put,
    path = "/api/v1/swap-requests/{id}",
    tag = "Bookings",
    summary = "Update swap request",
    description = "Accept or decline a swap request.",
    security(("bearer_auth" = []))
)]
pub async fn update_swap_request(
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
            let Ok(Some(mut requester_booking)) = state_guard
                .db
                .get_booking(&swap.requester_booking_id.to_string())
                .await
            else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Requester booking not found",
                    )),
                );
            };

            let Ok(Some(mut target_booking)) = state_guard
                .db
                .get_booking(&swap.target_booking_id.to_string())
                .await
            else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Target booking not found",
                    )),
                );
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
#[utoipa::path(
    get,
    path = "/api/v1/recurring-bookings",
    tag = "Bookings",
    summary = "List recurring bookings",
    description = "List the current user's recurring booking patterns.",
    security(("bearer_auth" = []))
)]
pub async fn list_recurring_bookings(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateRecurringBookingRequest {
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
#[utoipa::path(
    post,
    path = "/api/v1/recurring-bookings",
    tag = "Bookings",
    summary = "Create recurring booking",
    description = "Create a new recurring booking pattern (e.g. every Tuesday 8-17).",
    security(("bearer_auth" = []))
)]
pub async fn create_recurring_booking(
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
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Delete recurring booking",
    description = "Delete a recurring booking pattern. Verifies ownership.",
    security(("bearer_auth" = []))
)]
pub async fn delete_recurring_booking(
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

    let Ok(id_uuid) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "Invalid ID format")),
        );
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateGuestBookingRequest {
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
#[utoipa::path(
    post,
    path = "/api/v1/bookings/guest",
    tag = "Bookings",
    summary = "Create guest booking",
    description = "Create a visitor parking booking with a guest code.",
    security(("bearer_auth" = []))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, guest_name = %req.guest_name))]
pub async fn create_guest_booking(
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
#[utoipa::path(
    get,
    path = "/api/v1/admin/guest-bookings",
    tag = "Admin",
    summary = "List guest bookings",
    description = "List all guest bookings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_list_guest_bookings(
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
#[utoipa::path(
    patch,
    path = "/api/v1/admin/guest-bookings/{id}/cancel",
    tag = "Admin",
    summary = "Cancel guest booking",
    description = "Cancel a guest booking by ID. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_cancel_guest_booking(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct QuickBookRequest {
    lot_id: Uuid,
    date: Option<String>,
    booking_type: Option<String>,
}

/// `POST /api/v1/bookings/quick` — quick book with auto-assigned slot
#[utoipa::path(post, path = "/api/v1/bookings/quick", tag = "Bookings",
    summary = "Quick book (auto-assign slot)",
    description = "Auto-picks an available slot and creates a booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id, lot_id = %req.lot_id))]
pub async fn quick_book(
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
        .unwrap_or_else(|| Vehicle {
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
        "half_day_am" | "half_day_pm" => {
            let start = now + TimeDelta::minutes(1);
            let end = start + TimeDelta::hours(4);
            (start, end)
        }
        _ => {
            // full_day default: 8 hours
            let start = now + TimeDelta::minutes(1);
            let end = start + TimeDelta::hours(8);
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

    let floor_name = lot_opt.as_ref().map_or_else(
        || "Level 1".to_string(),
        |lot| {
            lot.floors
                .iter()
                .find(|f| f.id == available_slot.floor_id)
                .map_or_else(|| "Level 1".to_string(), |f| f.name.clone())
        },
    );

    let hourly_rate = lot_opt
        .as_ref()
        .and_then(|lot| lot.pricing.rates.iter().find(|r| r.duration_minutes == 60))
        .map_or(2.0, |r| r.price);
    let daily_max_gs = lot_opt.as_ref().and_then(|lot| lot.pricing.daily_max);
    let lot_currency_gs = lot_opt
        .as_ref()
        .map(|lot| lot.pricing.currency.clone())
        .unwrap_or_else(|| "EUR".to_string());

    #[allow(clippy::cast_precision_loss)]
    let raw_price_gs = ((end_time - start_time).num_minutes() as f64 / 60.0) * hourly_rate;
    let base_price = if let Some(cap) = daily_max_gs {
        raw_price_gs.min(cap)
    } else {
        raw_price_gs
    };
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
            currency: lot_currency_gs,
            payment_status: PaymentStatus::Pending,
            payment_method: None,
        },
        created_at: now,
        updated_at: now,
        check_in_time: None,
        check_out_time: None,
        qr_code: Some(Uuid::new_v4().to_string()),
        notes: Some(format!("Quick book ({booking_type})")),
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

    // Update slot status — fail the booking if slot update fails to prevent double-booking
    let mut updated_slot = available_slot;
    updated_slot.status = SlotStatus::Reserved;
    if let Err(e) = state_guard.db.save_parking_slot(&updated_slot).await {
        tracing::error!("Failed to update slot status after quick booking: {}", e);
        // Roll back the booking to avoid inconsistent state
        let _ = state_guard.db.delete_booking(&booking.id.to_string()).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SLOT_UPDATE_FAILED",
                "Failed to reserve slot",
            )),
        );
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
pub struct CalendarQuery {
    from: Option<String>,
    to: Option<String>,
}

/// Calendar event response
#[derive(Debug, Serialize)]
pub struct CalendarEvent {
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
#[utoipa::path(get, path = "/api/v1/calendar/events", tag = "Calendar",
    summary = "Calendar events",
    description = "Returns bookings and absences as calendar events.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn calendar_events(
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
pub struct AdminStatsResponse {
    total_users: u64,
    total_lots: u64,
    total_slots: u64,
    total_bookings: u64,
    active_bookings: u64,
    occupancy_percent: f64,
}

/// `GET /api/v1/admin/stats` — dashboard stats
#[utoipa::path(get, path = "/api/v1/admin/stats", tag = "Admin",
    summary = "Admin dashboard statistics",
    description = "Returns aggregated system stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[tracing::instrument(skip(state), fields(admin_id = %auth_user.user_id))]
pub async fn admin_stats(
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

    #[allow(clippy::cast_precision_loss)]
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
pub struct ReportsQuery {
    days: Option<i64>,
}

/// Booking stats by day
#[derive(Debug, Serialize)]
pub struct DailyBookingStat {
    date: String,
    count: usize,
}

/// `GET /api/v1/admin/reports` — booking stats by day for last N days
#[utoipa::path(get, path = "/api/v1/admin/reports", tag = "Admin",
    summary = "Booking reports (admin)",
    description = "Returns daily booking stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_reports(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ReportsQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<DailyBookingStat>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let days = query.days.unwrap_or(30);
    let cutoff = Utc::now() - TimeDelta::days(days);

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    // Group by date
    let mut by_date: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for b in &bookings {
        if b.created_at >= cutoff {
            let date = b.created_at.format("%Y-%m-%d").to_string();
            *by_date.entry(date).or_insert(0) += 1;
        }
    }

    let daily_stats: Vec<DailyBookingStat> = by_date
        .into_iter()
        .map(|(date, count)| DailyBookingStat { date, count })
        .collect();

    (StatusCode::OK, Json(ApiResponse::success(daily_stats)))
}

/// Heatmap cell: booking count by weekday x hour
#[derive(Debug, Serialize)]
pub struct HeatmapCell {
    weekday: u32,
    hour: u32,
    count: usize,
}

/// `GET /api/v1/admin/heatmap` — booking counts by weekday x hour
#[utoipa::path(get, path = "/api/v1/admin/heatmap", tag = "Admin",
    summary = "Booking heatmap (admin)",
    description = "Returns booking counts by weekday and hour.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_heatmap(
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
                    weekday: u32::try_from(wd).unwrap_or(0),
                    hour: u32::try_from(h).unwrap_or(0),
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
#[utoipa::path(get, path = "/api/v1/admin/audit-log", tag = "Admin",
    summary = "Audit log (admin)",
    description = "Returns recent audit log entries.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_audit_log(
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
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list audit log",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TEAM LIST
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct TeamMember {
    id: Uuid,
    name: String,
    username: String,
    role: String,
    is_active: bool,
}

/// `GET /api/v1/team` — list all team members (simplified view)
#[utoipa::path(get, path = "/api/v1/team", tag = "Team",
    summary = "List team members",
    description = "Returns all active team members.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn team_list(
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
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

/// `PATCH /api/v1/users/me/password` — authenticated user changes their own password
#[utoipa::path(patch, path = "/api/v1/users/me/password", tag = "Users",
    summary = "Change password",
    description = "Changes the authenticated user password.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
#[tracing::instrument(skip(state, req), fields(user_id = %auth_user.user_id))]
pub async fn change_password(
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
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update password",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// iCAL EXPORT
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/calendar.ics` — export user's bookings as iCal
#[utoipa::path(get, path = "/api/v1/user/calendar.ics", tag = "Calendar",
    summary = "Export bookings as iCal",
    description = "Returns bookings in iCalendar format.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn user_calendar_ics(
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
        let lot_name = match state_guard.db.get_parking_lot(&b.lot_id.to_string()).await {
            Ok(Some(l)) => l.name,
            _ => "Unknown Lot".to_string(),
        };

        let dtstart = b.start_time.format("%Y%m%dT%H%M%SZ");
        let dtend = b.end_time.format("%Y%m%dT%H%M%SZ");

        ical.push_str("BEGIN:VEVENT\r\n");
        let _ = write!(ical, "UID:{}@parkhub\r\n", b.id);
        let _ = write!(ical, "DTSTART:{dtstart}\r\n");
        let _ = write!(ical, "DTEND:{dtend}\r\n");
        let _ = write!(
            ical,
            "SUMMARY:Parking - {} - Slot {}\r\n",
            lot_name, b.slot_number
        );
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

// ═══════════════════════════════════════════════════════════════════════════════
// USER STATS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/stats` — authenticated user's personal statistics
#[utoipa::path(get, path = "/api/v1/user/stats", tag = "Users",
    summary = "User personal statistics",
    description = "Returns personal parking stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn user_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    let uid = auth_user.user_id.to_string();

    let Ok(Some(user)) = state_guard.db.get_user(&uid).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        );
    };

    let bookings = state_guard
        .db
        .list_bookings_by_user(&uid)
        .await
        .unwrap_or_default();

    let total_bookings = bookings.len();
    let active_bookings = bookings
        .iter()
        .filter(|b| {
            matches!(
                b.status,
                BookingStatus::Confirmed | BookingStatus::Active | BookingStatus::Pending
            )
        })
        .count();
    let cancelled_bookings = bookings
        .iter()
        .filter(|b| b.status == BookingStatus::Cancelled)
        .count();

    // Sum credits spent from deduction transactions
    let total_credits_spent = state_guard
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default()
        .iter()
        .filter(|tx| tx.transaction_type == CreditTransactionType::Deduction)
        .map(|tx| i64::from(tx.amount.abs()))
        .sum::<i64>();

    // Find favorite lot by most bookings
    let favorite_lot = {
        let mut lot_counts: std::collections::HashMap<Uuid, usize> =
            std::collections::HashMap::new();
        for b in &bookings {
            *lot_counts.entry(b.lot_id).or_insert(0) += 1;
        }
        if let Some((&lot_id, _)) = lot_counts.iter().max_by_key(|(_, &c)| c) {
            state_guard
                .db
                .get_parking_lot(&lot_id.to_string())
                .await
                .ok()
                .flatten()
                .map_or_else(|| "Unknown".to_string(), |l| l.name)
        } else {
            "None".to_string()
        }
    };

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "total_bookings": total_bookings,
            "active_bookings": active_bookings,
            "cancelled_bookings": cancelled_bookings,
            "total_credits_spent": total_credits_spent,
            "favorite_lot": favorite_lot,
            "member_since": user.created_at,
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// USER PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/preferences` — return current user's preferences
#[utoipa::path(get, path = "/api/v1/user/preferences", tag = "Users",
    summary = "Get user preferences",
    description = "Returns user preferences.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn get_user_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(user)) => (
            StatusCode::OK,
            Json(ApiResponse::success(serde_json::json!({
                "language": user.preferences.language,
                "theme": user.preferences.theme,
                "notifications_enabled": user.preferences.notifications_enabled,
                "email_reminders": user.preferences.email_reminders,
                "default_duration_minutes": user.preferences.default_duration_minutes,
            }))),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        ),
    }
}

/// Request body for updating user preferences
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdatePreferencesRequest {
    language: Option<String>,
    theme: Option<String>,
    notifications_enabled: Option<bool>,
    email_reminders: Option<bool>,
    default_duration_minutes: Option<i32>,
}

/// `PUT /api/v1/user/preferences` — update preferences
#[utoipa::path(put, path = "/api/v1/user/preferences", tag = "Users",
    summary = "Update user preferences",
    description = "Updates user preferences.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn update_user_preferences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    let Ok(Some(mut user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        );
    };

    if let Some(lang) = req.language {
        user.preferences.language = lang;
    }
    if let Some(theme) = req.theme {
        user.preferences.theme = theme;
    }
    if let Some(notif) = req.notifications_enabled {
        user.preferences.notifications_enabled = notif;
    }
    if let Some(email) = req.email_reminders {
        user.preferences.email_reminders = email;
    }
    if let Some(dur) = req.default_duration_minutes {
        user.preferences.default_duration_minutes = Some(dur);
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to save preferences: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save preferences",
            )),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "language": user.preferences.language,
            "theme": user.preferences.theme,
            "notifications_enabled": user.preferences.notifications_enabled,
            "email_reminders": user.preferences.email_reminders,
            "default_duration_minutes": user.preferences.default_duration_minutes,
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING CHECKIN
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/bookings/{id}/checkin` — mark booking as checked in
#[utoipa::path(post, path = "/api/v1/bookings/{id}/checkin", tag = "Bookings",
    summary = "Check in to a booking",
    description = "Marks a booking as checked-in.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn booking_checkin(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.write().await;

    let mut booking = match state_guard.db.get_booking(&id).await {
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

    // Only booking owner or admin can check in
    if booking.user_id != auth_user.user_id {
        if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
            return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
        }
    }

    // Only Confirmed or Pending bookings can be checked in
    if booking.status != BookingStatus::Confirmed && booking.status != BookingStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "INVALID_STATUS",
                "Only confirmed or pending bookings can be checked in",
            )),
        );
    }

    booking.status = BookingStatus::Active;
    booking.check_in_time = Some(Utc::now());
    booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to save booking checkin: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to check in booking",
            )),
        );
    }

    AuditEntry::new(AuditEventType::BookingUpdated)
        .user(auth_user.user_id, "")
        .resource("booking", &id)
        .details(serde_json::json!({"action": "checkin"}))
        .log();

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: DATABASE RESET
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for database reset confirmation
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminResetRequest {
    confirm: String,
}

/// `POST /api/v1/admin/reset` — wipe all data (admin only)
#[utoipa::path(post, path = "/api/v1/admin/reset", tag = "Admin",
    summary = "Reset database (admin)",
    description = "Wipes all data. Destructive. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn admin_reset(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AdminResetRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if req.confirm != "RESET" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "CONFIRMATION_REQUIRED",
                "Body must contain {\"confirm\": \"RESET\"}",
            )),
        );
    }

    // Capture admin info before wipe
    let Ok(Some(admin)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to read admin user before reset",
            )),
        );
    };

    if let Err(e) = state_guard.db.clear_all_data().await {
        tracing::error!("Database reset failed: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to reset database",
            )),
        );
    }

    // Re-create the admin user who triggered the reset
    let admin_user = User {
        id: admin.id,
        username: admin.username.clone(),
        email: admin.email.clone(),
        name: admin.name.clone(),
        password_hash: admin.password_hash,
        role: admin.role,
        is_active: true,
        phone: admin.phone,
        picture: admin.picture,
        preferences: admin.preferences,
        credits_balance: 0,
        credits_monthly_quota: 0,
        credits_last_refilled: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_login: None,
    };

    if let Err(e) = state_guard.db.save_user(&admin_user).await {
        tracing::error!("Failed to re-create admin after reset: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Database reset succeeded but admin re-creation failed",
            )),
        );
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, &admin_user.username)
        .details(serde_json::json!({"action": "database_reset"}))
        .log();

    tracing::warn!(
        admin = %admin_user.username,
        "Database reset completed"
    );

    (StatusCode::OK, Json(ApiResponse::success(())))
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: AUTO-RELEASE SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/settings/auto-release` — return auto-release config
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/auto-release",
    tag = "Admin",
    summary = "Get auto-release settings",
    description = "Return the auto-release timing configuration. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_auto_release(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let enabled = read_admin_setting(&state_guard.db, "auto_release_enabled").await;
    let minutes = read_admin_setting(&state_guard.db, "auto_release_minutes").await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "auto_release_enabled": enabled.parse::<bool>().unwrap_or(false),
            "auto_release_minutes": minutes.parse::<i32>().unwrap_or(30),
        }))),
    )
}

/// Request body for auto-release settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AutoReleaseSettingsRequest {
    auto_release_enabled: Option<bool>,
    auto_release_minutes: Option<i32>,
}

/// `PUT /api/v1/admin/settings/auto-release` — update auto-release timing
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/auto-release",
    tag = "Admin",
    summary = "Update auto-release settings",
    description = "Update auto-release timing for unclaimed bookings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_auto_release(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AutoReleaseSettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if let Some(enabled) = req.auto_release_enabled {
        if let Err(e) = state_guard
            .db
            .set_setting("auto_release_enabled", &enabled.to_string())
            .await
        {
            tracing::error!("Failed to save auto_release_enabled: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }
    }

    if let Some(minutes) = req.auto_release_minutes {
        if minutes < 1 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "auto_release_minutes must be >= 1",
                )),
            );
        }
        if let Err(e) = state_guard
            .db
            .set_setting("auto_release_minutes", &minutes.to_string())
            .await
        {
            tracing::error!("Failed to save auto_release_minutes: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save setting")),
            );
        }
    }

    // Return updated values
    let enabled = read_admin_setting(&state_guard.db, "auto_release_enabled").await;
    let minutes = read_admin_setting(&state_guard.db, "auto_release_minutes").await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "auto_release_enabled": enabled.parse::<bool>().unwrap_or(false),
            "auto_release_minutes": minutes.parse::<i32>().unwrap_or(30),
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: EMAIL SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/settings/email` — return SMTP config (password masked)
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/email",
    tag = "Admin",
    summary = "Get email settings",
    description = "Return SMTP configuration (password masked). Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_email_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let host = state_guard
        .db
        .get_setting("smtp_host")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let port = state_guard
        .db
        .get_setting("smtp_port")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "587".to_string());
    let username = state_guard
        .db
        .get_setting("smtp_username")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let has_password = state_guard
        .db
        .get_setting("smtp_password")
        .await
        .ok()
        .flatten()
        .is_some_and(|p| !p.is_empty());
    let from = state_guard
        .db
        .get_setting("smtp_from")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let enabled = state_guard
        .db
        .get_setting("smtp_enabled")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "false".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "smtp_host": host,
            "smtp_port": port.parse::<i32>().unwrap_or(587),
            "smtp_username": username,
            "smtp_password": if has_password { "********" } else { "" },
            "smtp_from": from,
            "smtp_enabled": enabled.parse::<bool>().unwrap_or(false),
        }))),
    )
}

/// Request body for email settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EmailSettingsRequest {
    #[serde(alias = "smtp_host")]
    host: Option<String>,
    #[serde(alias = "smtp_port")]
    port: Option<i32>,
    #[serde(alias = "smtp_username")]
    username: Option<String>,
    #[serde(alias = "smtp_password")]
    password: Option<String>,
    #[serde(alias = "smtp_from")]
    from: Option<String>,
    #[serde(alias = "smtp_enabled")]
    enabled: Option<bool>,
}

/// `PUT /api/v1/admin/settings/email` — update SMTP settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/email",
    tag = "Admin",
    summary = "Update email settings",
    description = "Update SMTP settings for outgoing emails. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_email_settings(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<EmailSettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    if let Some(host) = &req.host {
        let _ = db.set_setting("smtp_host", host).await;
    }
    if let Some(port) = req.port {
        let _ = db.set_setting("smtp_port", &port.to_string()).await;
    }
    if let Some(username) = &req.username {
        let _ = db.set_setting("smtp_username", username).await;
    }
    if let Some(password) = &req.password {
        // Don't overwrite with the masked placeholder
        if password != "********" {
            let _ = db.set_setting("smtp_password", password).await;
        }
    }
    if let Some(from) = &req.from {
        let _ = db.set_setting("smtp_from", from).await;
    }
    if let Some(enabled) = req.enabled {
        let _ = db.set_setting("smtp_enabled", &enabled.to_string()).await;
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .resource("settings", "email")
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::json!({"message": "Email settings updated"}),
        )),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: PRIVACY SETTINGS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/privacy` — return privacy/GDPR settings
#[utoipa::path(
    get,
    path = "/api/v1/admin/privacy",
    tag = "Admin",
    summary = "Get privacy settings",
    description = "Return privacy and GDPR settings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_get_privacy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    let privacy_policy_url = db
        .get_setting("privacy_policy_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let data_retention_days = db
        .get_setting("data_retention_days")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "365".to_string());
    let require_consent = db
        .get_setting("require_consent")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());
    let anonymize_on_delete = db
        .get_setting("anonymize_on_delete")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "privacy_policy_url": privacy_policy_url,
            "data_retention_days": data_retention_days.parse::<i32>().unwrap_or(365),
            "require_consent": require_consent.parse::<bool>().unwrap_or(true),
            "anonymize_on_delete": anonymize_on_delete.parse::<bool>().unwrap_or(true),
        }))),
    )
}

/// Request body for privacy settings update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PrivacySettingsRequest {
    privacy_policy_url: Option<String>,
    data_retention_days: Option<i32>,
    require_consent: Option<bool>,
    anonymize_on_delete: Option<bool>,
}

/// `PUT /api/v1/admin/privacy` — update privacy settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/privacy",
    tag = "Admin",
    summary = "Update privacy settings",
    description = "Update privacy and GDPR settings. Admin only.",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_privacy(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<PrivacySettingsRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let db = &state_guard.db;

    if let Some(url) = &req.privacy_policy_url {
        let _ = db.set_setting("privacy_policy_url", url).await;
    }
    if let Some(days) = req.data_retention_days {
        if days < 1 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "data_retention_days must be >= 1",
                )),
            );
        }
        let _ = db
            .set_setting("data_retention_days", &days.to_string())
            .await;
    }
    if let Some(consent) = req.require_consent {
        let _ = db
            .set_setting("require_consent", &consent.to_string())
            .await;
    }
    if let Some(anonymize) = req.anonymize_on_delete {
        let _ = db
            .set_setting("anonymize_on_delete", &anonymize.to_string())
            .await;
    }

    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .resource("settings", "privacy")
        .log();

    // Return current state
    let privacy_policy_url = db
        .get_setting("privacy_policy_url")
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let data_retention_days = db
        .get_setting("data_retention_days")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "365".to_string());
    let require_consent = db
        .get_setting("require_consent")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());
    let anonymize_on_delete = db
        .get_setting("anonymize_on_delete")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "true".to_string());

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "privacy_policy_url": privacy_policy_url,
            "data_retention_days": data_retention_days.parse::<i32>().unwrap_or(365),
            "require_consent": require_consent.parse::<bool>().unwrap_or(true),
            "anonymize_on_delete": anonymize_on_delete.parse::<bool>().unwrap_or(true),
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADMIN: UPDATE USER
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for admin user update
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminUpdateUserRequest {
    name: Option<String>,
    email: Option<String>,
    role: Option<String>,
    is_active: Option<bool>,
}

/// `PUT /api/v1/admin/users/{id}/update` — admin can update user details
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/update",
    tag = "Admin",
    summary = "Update user details",
    description = "Admin can update any user's details (name, email, department, etc.).",
    security(("bearer_auth" = []))
)]
pub async fn admin_update_user(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<AdminUpdateUserRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
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

    if let Some(name) = req.name {
        user.name = name;
    }
    if let Some(email) = req.email {
        // Basic email validation
        if !email.contains('@') || email.len() < 5 {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_INPUT", "Invalid email address")),
            );
        }
        user.email = email;
    }
    if let Some(role_str) = req.role {
        let new_role = match role_str.to_lowercase().as_str() {
            "user" => UserRole::User,
            "premium" => UserRole::Premium,
            "admin" => UserRole::Admin,
            "superadmin" => {
                // Only SuperAdmin can assign SuperAdmin
                let caller = state_guard
                    .db
                    .get_user(&auth_user.user_id.to_string())
                    .await
                    .ok()
                    .flatten();
                if caller.map(|c| c.role) != Some(UserRole::SuperAdmin) {
                    return (
                        StatusCode::FORBIDDEN,
                        Json(ApiResponse::error(
                            "FORBIDDEN",
                            "Only SuperAdmin can assign SuperAdmin role",
                        )),
                    );
                }
                UserRole::SuperAdmin
            }
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        "INVALID_INPUT",
                        "Role must be user, premium, admin, or superadmin",
                    )),
                );
            }
        };
        user.role = new_role;
    }
    if let Some(active) = req.is_active {
        user.is_active = active;
    }
    user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&user).await {
        tracing::error!("Failed to update user: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update user")),
        );
    }

    AuditEntry::new(AuditEventType::UserUpdated)
        .user(auth_user.user_id, "admin")
        .resource("user", &id)
        .log();

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "id": user.id.to_string(),
            "username": user.username,
            "email": user.email,
            "name": user.name,
            "role": format!("{:?}", user.role).to_lowercase(),
            "is_active": user.is_active,
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING UPDATE (PATCH /api/v1/bookings/{id})
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for patching a booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PatchBookingRequest {
    pub notes: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// `PATCH /api/v1/bookings/{id}` — update notes/times on an existing booking
#[utoipa::path(
    patch,
    path = "/api/v1/bookings/{id}",
    tag = "Bookings",
    summary = "Update a booking",
    description = "Update notes and/or times on a booking. Only the booking owner or an admin may update.",
    security(("bearer_auth" = []))
)]
pub async fn update_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<PatchBookingRequest>,
) -> (StatusCode, Json<ApiResponse<Booking>>) {
    let state_guard = state.read().await;

    let mut booking = match state_guard.db.get_booking(&id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Check ownership or admin
    let caller = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };
    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if booking.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    if let Some(notes) = req.notes {
        booking.notes = Some(notes);
    }
    if let Some(start_time) = req.start_time {
        booking.start_time = start_time;
    }
    if let Some(end_time) = req.end_time {
        booking.end_time = end_time;
    }
    booking.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_booking(&booking).await {
        tracing::error!("Failed to update booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update booking")),
        );
    }

    AuditEntry::new(AuditEventType::BookingUpdated)
        .user(auth_user.user_id, &caller.username)
        .resource("booking", &id)
        .details(serde_json::json!({"action": "patch"}))
        .log();

    (StatusCode::OK, Json(ApiResponse::success(booking)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// RECURRING BOOKING UPDATE (PUT /api/v1/recurring-bookings/{id})
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for updating a recurring booking
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateRecurringBookingRequest {
    pub days_of_week: Option<Vec<u8>>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

/// `PUT /api/v1/recurring-bookings/{id}` — update a recurring booking pattern
#[utoipa::path(
    put,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Update a recurring booking",
    description = "Update days_of_week, start_date, or end_date of a recurring booking. Only the owner or an admin may update. Note: re-expansion of future bookings is not performed automatically.",
    security(("bearer_auth" = []))
)]
pub async fn update_recurring_booking(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRecurringBookingRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let Ok(id_uuid) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_ID", "Invalid ID format")),
        );
    };

    let state_guard = state.read().await;

    // Fetch caller to check admin status
    let caller = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };
    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;

    // Try ownership lookup first
    let user_bookings = state_guard
        .db
        .list_recurring_bookings_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();

    let mut booking = match user_bookings.into_iter().find(|b| b.id == id_uuid) {
        Some(b) => b,
        None => {
            if !is_admin {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NOT_FOUND",
                    "Recurring booking not found",
                )),
            );
        }
    };

    if let Some(days) = req.days_of_week {
        booking.days_of_week = days;
    }
    if let Some(start_date) = req.start_date {
        booking.start_date = start_date;
    }
    if let Some(end_date) = req.end_date {
        booking.end_date = Some(end_date);
    }

    if let Err(e) = state_guard.db.save_recurring_booking(&booking).await {
        tracing::error!("Failed to update recurring booking: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update recurring booking",
            )),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "id": booking.id.to_string(),
            "days_of_week": booking.days_of_week,
            "start_date": booking.start_date,
            "end_date": booking.end_date,
            "note": "Future expanded bookings are not re-generated automatically. Trigger re-expansion separately if needed."
        }))),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// ABSENCE UPDATE (PUT /api/v1/absences/{id})
// ═══════════════════════════════════════════════════════════════════════════════

/// Request body for updating an absence
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateAbsenceRequest {
    pub absence_type: Option<AbsenceType>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

/// `PUT /api/v1/absences/{id}` — update an existing absence
#[utoipa::path(
    put,
    path = "/api/v1/absences/{id}",
    tag = "Absences",
    summary = "Update an absence",
    description = "Update absence_type, start_date, end_date, or notes. Only the owner or an admin may update.",
    security(("bearer_auth" = []))
)]
pub async fn update_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<Absence>>) {
    let state_guard = state.read().await;

    let mut absence = match state_guard.db.get_absence(&id).await {
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

    // Check ownership or admin
    let caller = match state_guard.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Access denied")),
            );
        }
    };
    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if absence.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    if let Some(absence_type) = req.absence_type {
        absence.absence_type = absence_type;
    }
    if let Some(start_date) = req.start_date {
        if !is_valid_date(&start_date) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "start_date must be in YYYY-MM-DD format",
                )),
            );
        }
        absence.start_date = start_date;
    }
    if let Some(end_date) = req.end_date {
        if !is_valid_date(&end_date) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "end_date must be in YYYY-MM-DD format",
                )),
            );
        }
        absence.end_date = end_date;
    }
    if let Some(notes) = req.notes {
        absence.note = Some(notes);
    }

    if absence.start_date > absence.end_date {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "start_date must not be after end_date",
            )),
        );
    }

    match state_guard.db.save_absence(&absence).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(absence))),
        Err(e) => {
            tracing::error!("Failed to update absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to update absence")),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── normalize_metric_path ─────────────────────────────────────────

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

    // ─── is_valid_date ─────────────────────────────────────────────────

    #[test]
    fn test_is_valid_date_correct() {
        assert!(is_valid_date("2026-03-20"));
        assert!(is_valid_date("2000-01-01"));
        assert!(is_valid_date("2026-12-31"));
    }

    #[test]
    fn test_is_valid_date_invalid() {
        assert!(!is_valid_date("2026-13-01")); // month 13
        assert!(!is_valid_date("2026-02-30")); // Feb 30
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date(""));
        assert!(!is_valid_date("20260320"));
        assert!(!is_valid_date("2026/03/20"));
    }

    #[test]
    fn test_is_valid_date_leap_year() {
        assert!(is_valid_date("2024-02-29")); // 2024 is leap
        assert!(!is_valid_date("2025-02-29")); // 2025 is not
    }

    // ─── generate_guest_code ───────────────────────────────────────────

    #[test]
    fn test_generate_guest_code_length() {
        let code = generate_guest_code();
        assert_eq!(code.len(), 8);
    }

    #[test]
    fn test_generate_guest_code_charset() {
        let valid_chars: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        for _ in 0..20 {
            let code = generate_guest_code();
            for c in code.chars() {
                assert!(
                    valid_chars.contains(c),
                    "Invalid char '{}' in guest code",
                    c
                );
            }
        }
    }

    #[test]
    fn test_generate_guest_code_uniqueness() {
        let codes: Vec<String> = (0..50).map(|_| generate_guest_code()).collect();
        let unique: std::collections::HashSet<&String> = codes.iter().collect();
        // With 8 chars from 31-char set, collisions in 50 codes are astronomically unlikely
        assert!(unique.len() > 45);
    }

    // ─── ImpressumData serde ───────────────────────────────────────────

    #[test]
    fn test_impressum_data_default() {
        let data = ImpressumData::default();
        assert_eq!(data.provider_name, "");
        assert_eq!(data.country, "");
    }

    #[test]
    fn test_impressum_data_roundtrip() {
        let data = ImpressumData {
            provider_name: "ParkCorp GmbH".to_string(),
            provider_legal_form: "GmbH".to_string(),
            street: "Musterstr. 1".to_string(),
            zip_city: "12345 Berlin".to_string(),
            country: "DE".to_string(),
            email: "info@parkcorp.de".to_string(),
            phone: "+49 30 123456".to_string(),
            register_court: "Amtsgericht Berlin".to_string(),
            register_number: "HRB 12345".to_string(),
            vat_id: "DE123456789".to_string(),
            responsible_person: "Max Mustermann".to_string(),
            custom_text: "".to_string(),
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: ImpressumData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider_name, "ParkCorp GmbH");
        assert_eq!(back.vat_id, "DE123456789");
    }

    // ─── UpdateCurrentUserRequest serde ────────────────────────────────

    #[test]
    fn test_update_current_user_request_full() {
        let json =
            r#"{"name":"New Name","phone":"+49123","picture":"https://img.example/pic.jpg"}"#;
        let req: UpdateCurrentUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("New Name"));
        assert_eq!(req.phone.as_deref(), Some("+49123"));
        assert_eq!(req.picture.as_deref(), Some("https://img.example/pic.jpg"));
    }

    #[test]
    fn test_update_current_user_request_empty() {
        let json = r#"{}"#;
        let req: UpdateCurrentUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.phone.is_none());
        assert!(req.picture.is_none());
    }

    // ─── UpdateUserRoleRequest / UpdateUserStatusRequest ───────────────

    #[test]
    fn test_update_user_role_request() {
        let json = r#"{"role":"admin"}"#;
        let req: UpdateUserRoleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, "admin");
    }

    #[test]
    fn test_update_user_status_request() {
        let json = r#"{"status":"active"}"#;
        let req: UpdateUserStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "active");
    }

    // ─── ChangePasswordRequest ─────────────────────────────────────────

    #[test]
    fn test_change_password_request() {
        let json = r#"{"current_password":"old","new_password":"NewSecure123!"}"#;
        let req: ChangePasswordRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.current_password, "old");
        assert_eq!(req.new_password, "NewSecure123!");
    }

    // ─── JoinWaitlistRequest ───────────────────────────────────────────

    #[test]
    fn test_join_waitlist_request() {
        let json = r#"{"lot_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: JoinWaitlistRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.lot_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    // ─── CreateSwapRequestBody ─────────────────────────────────────────

    #[test]
    fn test_create_swap_request_body() {
        let json = r#"{"target_booking_id":"550e8400-e29b-41d4-a716-446655440000","message":"Please swap?"}"#;
        let req: CreateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.target_booking_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(req.message.as_deref(), Some("Please swap?"));
    }

    #[test]
    fn test_create_swap_request_body_no_message() {
        let json = r#"{"target_booking_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: CreateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert!(req.message.is_none());
    }

    // ─── UpdateSwapRequestBody ─────────────────────────────────────────

    #[test]
    fn test_update_swap_request_body_accept() {
        let json = r#"{"action":"accept"}"#;
        let req: UpdateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "accept");
    }

    #[test]
    fn test_update_swap_request_body_decline() {
        let json = r#"{"action":"decline"}"#;
        let req: UpdateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "decline");
    }

    // ─── CreateRecurringBookingRequest ─────────────────────────────────

    #[test]
    fn test_create_recurring_booking_request_full() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "days_of_week":[1,3,5],
            "start_date":"2026-04-01",
            "end_date":"2026-06-30",
            "start_time":"08:00",
            "end_time":"17:00",
            "vehicle_plate":"B-AB 1234"
        }"#;
        let req: CreateRecurringBookingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.days_of_week, vec![1, 3, 5]);
        assert_eq!(req.start_date, "2026-04-01");
        assert_eq!(req.end_date.as_deref(), Some("2026-06-30"));
        assert_eq!(req.vehicle_plate.as_deref(), Some("B-AB 1234"));
    }

    #[test]
    fn test_create_recurring_booking_request_minimal() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "days_of_week":[1],
            "start_date":"2026-04-01",
            "start_time":"09:00",
            "end_time":"18:00"
        }"#;
        let req: CreateRecurringBookingRequest = serde_json::from_str(json).unwrap();
        assert!(req.slot_id.is_none());
        assert!(req.end_date.is_none());
        assert!(req.vehicle_plate.is_none());
    }

    // ─── CreateGuestBookingRequest ─────────────────────────────────────

    #[test]
    fn test_create_guest_booking_request() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "start_time":"2026-04-01T08:00:00Z",
            "end_time":"2026-04-01T17:00:00Z",
            "guest_name":"Visitor One",
            "guest_email":"visitor@example.com"
        }"#;
        let req: CreateGuestBookingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.guest_name, "Visitor One");
        assert_eq!(req.guest_email.as_deref(), Some("visitor@example.com"));
    }

    #[test]
    fn test_create_guest_booking_request_no_email() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "slot_id":"660e8400-e29b-41d4-a716-446655440001",
            "start_time":"2026-04-01T08:00:00Z",
            "end_time":"2026-04-01T17:00:00Z",
            "guest_name":"Walk-in"
        }"#;
        let req: CreateGuestBookingRequest = serde_json::from_str(json).unwrap();
        assert!(req.guest_email.is_none());
    }

    // ─── AdminResetRequest ─────────────────────────────────────────────

    #[test]
    fn test_admin_reset_request() {
        let json = r#"{"confirm":"RESET"}"#;
        let req: AdminResetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.confirm, "RESET");
    }

    // ─── AutoReleaseSettingsRequest ────────────────────────────────────

    #[test]
    fn test_auto_release_settings_request() {
        let json = r#"{"auto_release_enabled":true,"auto_release_minutes":15}"#;
        let req: AutoReleaseSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.auto_release_enabled, Some(true));
        assert_eq!(req.auto_release_minutes, Some(15));
    }

    #[test]
    fn test_auto_release_settings_request_partial() {
        let json = r#"{"auto_release_minutes":45}"#;
        let req: AutoReleaseSettingsRequest = serde_json::from_str(json).unwrap();
        assert!(req.auto_release_enabled.is_none());
        assert_eq!(req.auto_release_minutes, Some(45));
    }

    // ─── EmailSettingsRequest ──────────────────────────────────────────

    #[test]
    fn test_email_settings_request_full() {
        let json = r#"{
            "smtp_host":"smtp.example.com",
            "smtp_port":587,
            "smtp_username":"user@example.com",
            "smtp_password":"secret",
            "smtp_from":"noreply@example.com",
            "smtp_enabled":true
        }"#;
        let req: EmailSettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.host.as_deref(), Some("smtp.example.com"));
        assert_eq!(req.port, Some(587));
        assert_eq!(req.enabled, Some(true));
    }

    #[test]
    fn test_email_settings_request_empty() {
        let json = r#"{}"#;
        let req: EmailSettingsRequest = serde_json::from_str(json).unwrap();
        assert!(req.host.is_none());
        assert!(req.port.is_none());
        assert!(req.enabled.is_none());
    }

    // ─── PrivacySettingsRequest ────────────────────────────────────────

    #[test]
    fn test_privacy_settings_request() {
        let json = r#"{
            "privacy_policy_url":"https://example.com/privacy",
            "data_retention_days":365,
            "require_consent":true,
            "anonymize_on_delete":true
        }"#;
        let req: PrivacySettingsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.privacy_policy_url.as_deref(),
            Some("https://example.com/privacy")
        );
        assert_eq!(req.data_retention_days, Some(365));
        assert_eq!(req.require_consent, Some(true));
        assert_eq!(req.anonymize_on_delete, Some(true));
    }

    // ─── AdminUpdateUserRequest ────────────────────────────────────────

    #[test]
    fn test_admin_update_user_request_full() {
        let json =
            r#"{"name":"Updated","email":"new@example.com","role":"admin","is_active":false}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("Updated"));
        assert_eq!(req.email.as_deref(), Some("new@example.com"));
        assert_eq!(req.role.as_deref(), Some("admin"));
        assert_eq!(req.is_active, Some(false));
    }

    #[test]
    fn test_admin_update_user_request_partial() {
        let json = r#"{"is_active":true}"#;
        let req: AdminUpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.name.is_none());
        assert!(req.email.is_none());
        assert!(req.role.is_none());
        assert_eq!(req.is_active, Some(true));
    }

    // ─── UpdateFeaturesRequest ─────────────────────────────────────────

    #[test]
    fn test_update_features_request() {
        let json = r#"{"enabled":["credits","absences","vehicles"]}"#;
        let req: UpdateFeaturesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled.len(), 3);
        assert!(req.enabled.contains(&"credits".to_string()));
    }

    #[test]
    fn test_update_features_request_empty() {
        let json = r#"{"enabled":[]}"#;
        let req: UpdateFeaturesRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled.is_empty());
    }

    // ─── CreateAbsenceRequest ──────────────────────────────────────────

    #[test]
    fn test_create_absence_request() {
        let json = r#"{"absence_type":"homeoffice","start_date":"2026-04-01","end_date":"2026-04-01","note":"WFH"}"#;
        let req: CreateAbsenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.start_date, "2026-04-01");
        assert_eq!(req.end_date, "2026-04-01");
        assert_eq!(req.note.as_deref(), Some("WFH"));
    }

    #[test]
    fn test_create_absence_request_no_note() {
        let json =
            r#"{"absence_type":"vacation","start_date":"2026-04-01","end_date":"2026-04-05"}"#;
        let req: CreateAbsenceRequest = serde_json::from_str(json).unwrap();
        assert!(req.note.is_none());
    }

    // ─── CreateAnnouncementRequest ─────────────────────────────────────

    #[test]
    fn test_create_announcement_request_full() {
        let json = r#"{
            "title":"Maintenance",
            "message":"Lot A closed on Monday",
            "severity":"warning",
            "active":true,
            "expires_at":"2026-04-01T00:00:00Z"
        }"#;
        let req: CreateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.title, "Maintenance");
        assert_eq!(req.message, "Lot A closed on Monday");
        assert_eq!(req.active, Some(true));
        assert!(req.expires_at.is_some());
    }

    #[test]
    fn test_create_announcement_request_minimal() {
        let json = r#"{"title":"Info","message":"Welcome!","severity":"info"}"#;
        let req: CreateAnnouncementRequest = serde_json::from_str(json).unwrap();
        assert!(req.active.is_none());
        assert!(req.expires_at.is_none());
    }

    // ─── UpdatePreferencesRequest ──────────────────────────────────────

    #[test]
    fn test_update_preferences_request() {
        let json = r#"{"language":"de","theme":"dark","notifications_enabled":false}"#;
        let req: UpdatePreferencesRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.language.as_deref(), Some("de"));
        assert_eq!(req.theme.as_deref(), Some("dark"));
        assert_eq!(req.notifications_enabled, Some(false));
        assert!(req.email_reminders.is_none());
        assert!(req.default_duration_minutes.is_none());
    }

    // ─── CalendarQuery / CalendarEvent ─────────────────────────────────

    #[test]
    fn test_calendar_query_deserialize() {
        let json = r#"{"from":"2026-03-01","to":"2026-03-31"}"#;
        let q: CalendarQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.from.as_deref(), Some("2026-03-01"));
        assert_eq!(q.to.as_deref(), Some("2026-03-31"));
    }

    #[test]
    fn test_calendar_event_serialize_skip_none() {
        let event = CalendarEvent {
            id: "evt-1".to_string(),
            event_type: "booking".to_string(),
            title: "Slot A3".to_string(),
            start: Utc::now(),
            end: Utc::now() + TimeDelta::hours(2),
            lot_name: None,
            slot_number: None,
            status: None,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("lot_name"));
        assert!(!json.contains("slot_number"));
        assert!(!json.contains("status"));
    }

    #[test]
    fn test_calendar_event_serialize_with_optionals() {
        let event = CalendarEvent {
            id: "evt-2".to_string(),
            event_type: "booking".to_string(),
            title: "Slot B1".to_string(),
            start: Utc::now(),
            end: Utc::now() + TimeDelta::hours(1),
            lot_name: Some("Lot Alpha".to_string()),
            slot_number: Some(42),
            status: Some("confirmed".to_string()),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Lot Alpha"));
        assert!(json.contains("42"));
        assert!(json.contains("confirmed"));
        // Check rename
        assert!(json.contains(r#""type":"booking"#));
    }

}
