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
pub mod favorites;
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
use admin::{
    admin_audit_log, admin_cancel_guest_booking, admin_create_announcement, read_admin_setting,
    admin_dashboard_charts, admin_delete_announcement, admin_delete_user,
    admin_get_auto_release, admin_get_email_settings, admin_get_features,
    admin_get_privacy, admin_get_settings, admin_get_use_case, admin_heatmap,
    admin_list_announcements, admin_list_bookings, admin_list_guest_bookings,
    admin_list_users, admin_reports, admin_reset, admin_stats,
    admin_update_announcement, admin_update_auto_release, admin_update_email_settings,
    admin_update_features, admin_update_privacy, admin_update_settings,
    admin_update_user, admin_update_user_role, admin_update_user_status,
    get_active_announcements, get_features, get_impressum, get_impressum_admin,
    get_public_theme, public_display, public_occupancy, update_impressum,
};
use auth::{forgot_password, login, refresh_token, register, reset_password};
use bookings::{
    booking_checkin, calendar_events, cancel_booking, create_booking, create_guest_booking,
    create_recurring_booking, delete_recurring_booking, get_booking, get_booking_invoice,
    list_bookings, list_recurring_bookings, quick_book, update_booking, update_recurring_booking,
};
use credits::{
    admin_grant_credits, admin_list_credit_transactions, admin_refill_all_credits,
    admin_update_user_quota, get_user_credits,
};
use export::{admin_export_bookings_csv, admin_export_revenue_csv, admin_export_users_csv};
use favorites::{add_favorite, list_favorites, remove_favorite};
use import::import_users_csv;
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_pricing, get_lot_slots,
    list_lots, update_lot, update_lot_pricing, update_slot,
};
use qr::lot_qr_code;
use recommendations::get_recommendations;
use social::{
    create_absence, create_swap_request, delete_absence, get_absence_pattern, join_waitlist,
    leave_waitlist, list_absences, list_notifications, list_swap_requests, list_team_absences,
    list_waitlist, mark_all_notifications_read, mark_notification_read, save_absence_pattern,
    team_list, team_today, update_absence, update_swap_request,
};
use translations::{
    create_proposal, get_proposal, list_overrides, list_proposals, review_proposal,
    vote_on_proposal,
};
use users::{
    change_password, gdpr_delete_account, gdpr_export_data, get_current_user, get_user,
    get_user_preferences, update_current_user, update_user_preferences, user_calendar_ics,
    user_stats,
};
use vehicles::{
    create_vehicle, delete_vehicle, get_vehicle_photo, list_vehicles, update_vehicle,
    upload_vehicle_photo, vehicle_city_codes,
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
// ═══════════════════════════════════════════════════════════════════════════════
// BOOKINGS
// ═══════════════════════════════════════════════════════════════════════════════

#[utoipa::path(get, path = "/api/v1/bookings", tag = "Bookings",
    summary = "List current user's bookings",
    description = "Returns all bookings for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "List of bookings"))

#[utoipa::path(post, path = "/api/v1/bookings", tag = "Bookings",
    summary = "Create a new booking",
    description = "Books a parking slot for the authenticated user.",
    security(("bearer_auth" = [])),
    request_body = CreateBookingRequest,
    responses((status = 201, description = "Booking created"), (status = 404, description = "Not found"), (status = 409, description = "Slot unavailable"))

#[utoipa::path(get, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Get booking by ID",
    description = "Returns a single booking. Only the owner can access it.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Booking found"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))

#[utoipa::path(delete, path = "/api/v1/bookings/{id}", tag = "Bookings",
    summary = "Cancel a booking",
    description = "Cancels an active booking and releases the slot.",
    security(("bearer_auth" = [])),
    params(("id" = String, Path, description = "Booking UUID")),
    responses((status = 200, description = "Cancelled"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))

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


/// `GET /api/v1/lots/{id}/qr` — generate a QR code URL for a parking lot.
///
/// Returns a URL pointing to an external QR API (api.qrserver.com) that renders
/// a QR code linking to the lot's booking page.
#[utoipa::path(get, path = "/api/v1/lots/{id}/qr", tag = "Lots",
    summary = "Generate QR code URL for a lot",
    description = "Returns a URL to an external QR service encoding the lot's booking page.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "Lot UUID")),
    responses((status = 200, description = "QR URLs"), (status = 404, description = "Lot not found"))


/// `GET /api/v1/admin/dashboard/charts` — aggregated chart data for the admin
/// dashboard.  Returns bookings-by-day (last 30 days), bookings-by-lot,
/// average occupancy by hour-of-day, and top-10 users by booking count.
#[utoipa::path(get, path = "/api/v1/admin/dashboard/charts", tag = "Admin",
    summary = "Admin dashboard chart data",
    description = "Returns aggregated chart data for the admin dashboard.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Chart data"), (status = 403, description = "Forbidden"))

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

/// Admin: read Impressum settings (admin-only, protected).
///
/// Although the public endpoint exposes the same data, this route is kept
/// separate so admins can fetch the current values before editing them via PUT.
/// It is deliberately restricted to Admin/SuperAdmin.
#[utoipa::path(get, path = "/api/v1/admin/impressum", tag = "Admin",
    summary = "Get Impressum settings (admin)", description = "Returns current Impressum fields for editing. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Impressum fields"), (status = 403, description = "Forbidden"))

/// Admin: update Impressum settings
#[utoipa::path(put, path = "/api/v1/admin/impressum", tag = "Admin",
    summary = "Update Impressum (admin)", description = "Saves DDG paragraph 5 Impressum fields. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Saved"), (status = 403, description = "Forbidden"))

// ═══════════════════════════════════════════════════════════════════════════════
// GDPR — Art. 15 (Data Export) + Art. 17 (Right to Erasure)
// ═══════════════════════════════════════════════════════════════════════════════

/// GDPR Art. 15 — Export all personal data for the authenticated user
#[utoipa::path(get, path = "/api/v1/users/me/export", tag = "Users",
    summary = "GDPR data export (Art. 15)", description = "Exports all personal data as JSON download.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "JSON data export"))

/// GDPR Art. 17 — Right to Erasure: anonymize user data, keep booking records for accounting.
/// Removes PII (name, email, username, password, vehicles) while preserving anonymized booking
/// records as required by German tax law (§ 147 AO — 10-year retention for accounting records).
#[utoipa::path(delete, path = "/api/v1/users/me/delete", tag = "Users",
    summary = "GDPR account deletion (Art. 17)", description = "Anonymizes user PII while preserving booking records.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Account anonymized"), (status = 404, description = "Not found"))


use admin::AdminUserResponse;

/// `GET /api/v1/admin/users` — list all users (admin only)
#[utoipa::path(get, path = "/api/v1/admin/users", tag = "Admin",
    summary = "List all users (admin)", description = "Returns all registered users. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "User list"), (status = 403, description = "Forbidden"))

/// `PATCH /api/v1/admin/users/{id}/role` — update a user's role (admin only)
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/role", tag = "Admin",
    summary = "Update user role (admin)", description = "Changes a user's role. Prevents privilege escalation.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Role updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))

/// `PATCH /api/v1/admin/users/{id}/status` — enable or disable a user account (admin only)
#[utoipa::path(patch, path = "/api/v1/admin/users/{id}/status", tag = "Admin",
    summary = "Enable or disable a user (admin)", description = "Sets a user's active/inactive status. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Updated"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))

/// `DELETE /api/v1/admin/users/{id}` — delete a user account (admin only, GDPR anonymize)
#[utoipa::path(delete, path = "/api/v1/admin/users/{id}", tag = "Admin",
    summary = "Delete user (admin)", description = "Anonymizes user data per GDPR. Admin only.",
    security(("bearer_auth" = [])), params(("id" = String, Path, description = "User UUID")),
    responses((status = 200, description = "Deleted"), (status = 403, description = "Forbidden"), (status = 404, description = "Not found"))


/// `GET /api/v1/admin/bookings` — list all bookings (admin only)
#[utoipa::path(get, path = "/api/v1/admin/bookings", tag = "Admin",
    summary = "List all bookings (admin)", description = "Returns all bookings with enriched details. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "All bookings"), (status = 403, description = "Forbidden"))

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

/// `GET /api/v1/admin/settings/use-case` — return current use-case with theme config
#[utoipa::path(
    get,
    path = "/api/v1/admin/settings/use-case",
    tag = "Admin",
    summary = "Get use-case configuration",
    description = "Return current use-case with theme config. Admin only.",
    security(("bearer_auth" = []))

/// `GET /api/v1/admin/settings` — return all settings (merged defaults + stored values)
#[utoipa::path(get, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Get system settings (admin)", description = "Returns all system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Settings"), (status = 403, description = "Forbidden"))

/// `PUT /api/v1/admin/settings` — update one or more settings (admin only)
#[utoipa::path(put, path = "/api/v1/admin/settings", tag = "Admin",
    summary = "Update system settings (admin)", description = "Saves system settings. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Saved"), (status = 403, description = "Forbidden"))

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

/// `GET /api/v1/features` — public endpoint returning enabled features
#[utoipa::path(get, path = "/api/v1/features", tag = "Public",
    summary = "Get enabled feature flags",
    description = "Returns enabled and available features. No auth required.",
    responses((status = 200, description = "Success"))

/// `GET /api/v1/theme` — public: return current use-case theme (no auth required)
#[utoipa::path(get, path = "/api/v1/theme", tag = "Public",
    summary = "Get current theme",
    description = "Returns theme and company name. No auth required.",
    responses((status = 200, description = "Success"))

/// `GET /api/v1/admin/features` — admin: get features with full metadata
#[utoipa::path(get, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Get feature flags (admin)",
    description = "Returns feature modules with status. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `PUT /api/v1/admin/features` — admin: update enabled features
#[utoipa::path(put, path = "/api/v1/admin/features", tag = "Admin",
    summary = "Update feature flags (admin)",
    description = "Sets enabled feature modules. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `GET /api/v1/absences` — list current user's absences, optionally filtered by type
#[utoipa::path(get, path = "/api/v1/absences", tag = "Absences",
    summary = "List user absences",
    description = "Returns absences for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// Validate a date string is YYYY-MM-DD format.
pub(crate) fn is_valid_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// `POST /api/v1/absences` — create an absence
#[utoipa::path(post, path = "/api/v1/absences", tag = "Absences",
    summary = "Create an absence",
    description = "Records a new absence for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `DELETE /api/v1/absences/{id}` — delete own absence
#[utoipa::path(delete, path = "/api/v1/absences/{id}", tag = "Absences",
    summary = "Delete an absence",
    description = "Removes an absence owned by the user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `GET /api/v1/absences/team` — list all team absences
#[utoipa::path(
    get,
    path = "/api/v1/absences/team",
    tag = "Absences",
    summary = "List team absences",
    description = "List all team member absences visible to the current user.",
    security(("bearer_auth" = []))

/// `GET /api/v1/absences/pattern` — get user's absence pattern
#[utoipa::path(
    get,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Get absence pattern",
    description = "Get the current user's recurring absence pattern.",
    security(("bearer_auth" = []))

/// `POST /api/v1/absences/pattern` — save user's absence pattern
#[utoipa::path(
    post,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Save absence pattern",
    description = "Save or update the current user's recurring absence pattern (e.g. homeoffice every Monday).",
    security(("bearer_auth" = []))


/// `GET /api/v1/team/today` — return all users with their status today
#[utoipa::path(get, path = "/api/v1/team/today", tag = "Team",
    summary = "Team status today",
    description = "Returns all users with their status for today.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// ANNOUNCEMENTS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/announcements/active` — public, return active non-expired announcements
#[utoipa::path(get, path = "/api/v1/announcements/active", tag = "Public",
    summary = "Get active announcements",
    description = "Returns active non-expired announcements. No auth required.",
    responses((status = 200, description = "Success"))

/// `GET /api/v1/admin/announcements` — admin: list all announcements
#[utoipa::path(get, path = "/api/v1/admin/announcements", tag = "Admin",
    summary = "List all announcements (admin)",
    description = "Returns all announcements. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `POST /api/v1/admin/announcements` — admin: create announcement
#[utoipa::path(
    post,
    path = "/api/v1/admin/announcements",
    tag = "Admin",
    summary = "Create announcement",
    description = "Create a new system announcement. Admin only.",
    security(("bearer_auth" = []))

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

/// `PUT /api/v1/admin/announcements/{id}` — admin: update announcement
#[utoipa::path(
    put,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Update announcement",
    description = "Update an existing announcement by ID. Admin only.",
    security(("bearer_auth" = []))

/// `DELETE /api/v1/admin/announcements/{id}` — admin: delete announcement
#[utoipa::path(
    delete,
    path = "/api/v1/admin/announcements/{id}",
    tag = "Admin",
    summary = "Delete announcement",
    description = "Delete an announcement by ID. Admin only.",
    security(("bearer_auth" = []))

// ═══════════════════════════════════════════════════════════════════════════════
// NOTIFICATIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/notifications` — list current user's notifications (most recent 50)
#[utoipa::path(get, path = "/api/v1/notifications", tag = "Notifications",
    summary = "List user notifications",
    description = "Returns recent notifications for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `PUT /api/v1/notifications/{id}/read` — mark notification as read (verify ownership)
#[utoipa::path(put, path = "/api/v1/notifications/{id}/read", tag = "Notifications",
    summary = "Mark notification as read",
    description = "Marks a notification as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `POST /api/v1/notifications/read-all` — mark all user's notifications as read
#[utoipa::path(post, path = "/api/v1/notifications/read-all", tag = "Notifications",
    summary = "Mark all notifications as read",
    description = "Marks all notifications as read.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// WAITLIST
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/waitlist` — list current user's waitlist entries
#[utoipa::path(get, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "List waitlist entries",
    description = "Returns waitlist entries for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `POST /api/v1/waitlist` — join waitlist for a lot
#[utoipa::path(post, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "Join waitlist",
    description = "Adds the user to a lot waitlist.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `DELETE /api/v1/waitlist/{id}` — leave waitlist (verify ownership)
#[utoipa::path(delete, path = "/api/v1/waitlist/{id}", tag = "Waitlist",
    summary = "Leave waitlist",
    description = "Removes the user from a waitlist entry.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

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

/// `POST /api/v1/bookings/{id}/swap-request` — create a swap request
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/swap-request",
    tag = "Bookings",
    summary = "Create swap request",
    description = "Create a parking slot swap request for a booking.",
    security(("bearer_auth" = []))

/// `PUT /api/v1/swap-requests/{id}` — accept or decline a swap request
#[utoipa::path(
    put,
    path = "/api/v1/swap-requests/{id}",
    tag = "Bookings",
    summary = "Update swap request",
    description = "Accept or decline a swap request.",
    security(("bearer_auth" = []))

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

/// `POST /api/v1/recurring-bookings` — create a recurring booking
#[utoipa::path(
    post,
    path = "/api/v1/recurring-bookings",
    tag = "Bookings",
    summary = "Create recurring booking",
    description = "Create a new recurring booking pattern (e.g. every Tuesday 8-17).",
    security(("bearer_auth" = []))

/// `DELETE /api/v1/recurring-bookings/{id}` — delete recurring booking (verify ownership)
#[utoipa::path(
    delete,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Delete recurring booking",
    description = "Delete a recurring booking pattern. Verifies ownership.",
    security(("bearer_auth" = []))


/// `POST /api/v1/bookings/guest` — create a guest booking
#[utoipa::path(
    post,
    path = "/api/v1/bookings/guest",
    tag = "Bookings",
    summary = "Create guest booking",
    description = "Create a visitor parking booking with a guest code.",
    security(("bearer_auth" = []))

/// `GET /api/v1/admin/guest-bookings` — admin: list all guest bookings
#[utoipa::path(
    get,
    path = "/api/v1/admin/guest-bookings",
    tag = "Admin",
    summary = "List guest bookings",
    description = "List all guest bookings. Admin only.",
    security(("bearer_auth" = []))

/// `PATCH /api/v1/admin/guest-bookings/{id}/cancel` — admin: cancel a guest booking
#[utoipa::path(
    patch,
    path = "/api/v1/admin/guest-bookings/{id}/cancel",
    tag = "Admin",
    summary = "Cancel guest booking",
    description = "Cancel a guest booking by ID. Admin only.",
    security(("bearer_auth" = []))


/// `POST /api/v1/bookings/quick` — quick book with auto-assigned slot
#[utoipa::path(post, path = "/api/v1/bookings/quick", tag = "Bookings",
    summary = "Quick book (auto-assign slot)",
    description = "Auto-picks an available slot and creates a booking.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `GET /api/v1/calendar/events` — return user's bookings + absences as calendar events
#[utoipa::path(get, path = "/api/v1/calendar/events", tag = "Calendar",
    summary = "Calendar events",
    description = "Returns bookings and absences as calendar events.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `GET /api/v1/admin/stats` — dashboard stats
#[utoipa::path(get, path = "/api/v1/admin/stats", tag = "Admin",
    summary = "Admin dashboard statistics",
    description = "Returns aggregated system stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `GET /api/v1/admin/reports` — booking stats by day for last N days
#[utoipa::path(get, path = "/api/v1/admin/reports", tag = "Admin",
    summary = "Booking reports (admin)",
    description = "Returns daily booking stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `GET /api/v1/admin/heatmap` — booking counts by weekday x hour
#[utoipa::path(get, path = "/api/v1/admin/heatmap", tag = "Admin",
    summary = "Booking heatmap (admin)",
    description = "Returns booking counts by weekday and hour.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// AUDIT LOG
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/audit-log` — list recent audit entries
#[utoipa::path(get, path = "/api/v1/admin/audit-log", tag = "Admin",
    summary = "Audit log (admin)",
    description = "Returns recent audit log entries.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `GET /api/v1/team` — list all team members (simplified view)
#[utoipa::path(get, path = "/api/v1/team", tag = "Team",
    summary = "List team members",
    description = "Returns all active team members.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `PATCH /api/v1/users/me/password` — authenticated user changes their own password
#[utoipa::path(patch, path = "/api/v1/users/me/password", tag = "Users",
    summary = "Change password",
    description = "Changes the authenticated user password.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// iCAL EXPORT
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/calendar.ics` — export user's bookings as iCal
#[utoipa::path(get, path = "/api/v1/user/calendar.ics", tag = "Calendar",
    summary = "Export bookings as iCal",
    description = "Returns bookings in iCalendar format.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `GET /api/v1/public/occupancy` — public JSON occupancy data
#[utoipa::path(get, path = "/api/v1/public/occupancy", tag = "Public",
    summary = "Public lot occupancy",
    description = "Returns real-time occupancy. No auth required.",
    responses((status = 200, description = "Success"))

/// `GET /api/v1/public/display` — simplified HTML for parking displays
#[utoipa::path(get, path = "/api/v1/public/display", tag = "Public",
    summary = "Public display HTML",
    description = "Returns minimal HTML for digital signage.",
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// USER STATS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/stats` — authenticated user's personal statistics
#[utoipa::path(get, path = "/api/v1/user/stats", tag = "Users",
    summary = "User personal statistics",
    description = "Returns personal parking stats.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// USER PREFERENCES
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/user/preferences` — return current user's preferences
#[utoipa::path(get, path = "/api/v1/user/preferences", tag = "Users",
    summary = "Get user preferences",
    description = "Returns user preferences.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

/// `PUT /api/v1/user/preferences` — update preferences
#[utoipa::path(put, path = "/api/v1/user/preferences", tag = "Users",
    summary = "Update user preferences",
    description = "Updates user preferences.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING CHECKIN
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/bookings/{id}/checkin` — mark booking as checked in
#[utoipa::path(post, path = "/api/v1/bookings/{id}/checkin", tag = "Bookings",
    summary = "Check in to a booking",
    description = "Marks a booking as checked-in.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))


/// `POST /api/v1/admin/reset` — wipe all data (admin only)
#[utoipa::path(post, path = "/api/v1/admin/reset", tag = "Admin",
    summary = "Reset database (admin)",
    description = "Wipes all data. Destructive. Admin only.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))

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

/// `PUT /api/v1/admin/settings/auto-release` — update auto-release timing
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/auto-release",
    tag = "Admin",
    summary = "Update auto-release settings",
    description = "Update auto-release timing for unclaimed bookings. Admin only.",
    security(("bearer_auth" = []))

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

/// `PUT /api/v1/admin/settings/email` — update SMTP settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/settings/email",
    tag = "Admin",
    summary = "Update email settings",
    description = "Update SMTP settings for outgoing emails. Admin only.",
    security(("bearer_auth" = []))

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

/// `PUT /api/v1/admin/privacy` — update privacy settings
#[utoipa::path(
    put,
    path = "/api/v1/admin/privacy",
    tag = "Admin",
    summary = "Update privacy settings",
    description = "Update privacy and GDPR settings. Admin only.",
    security(("bearer_auth" = []))


/// `PUT /api/v1/admin/users/{id}/update` — admin can update user details
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/update",
    tag = "Admin",
    summary = "Update user details",
    description = "Admin can update any user's details (name, email, department, etc.).",
    security(("bearer_auth" = []))


/// `PATCH /api/v1/bookings/{id}` — update notes/times on an existing booking
#[utoipa::path(
    patch,
    path = "/api/v1/bookings/{id}",
    tag = "Bookings",
    summary = "Update a booking",
    description = "Update notes and/or times on a booking. Only the booking owner or an admin may update.",
    security(("bearer_auth" = []))


/// `PUT /api/v1/recurring-bookings/{id}` — update a recurring booking pattern
#[utoipa::path(
    put,
    path = "/api/v1/recurring-bookings/{id}",
    tag = "Bookings",
    summary = "Update a recurring booking",
    description = "Update days_of_week, start_date, or end_date of a recurring booking. Only the owner or an admin may update. Note: re-expansion of future bookings is not performed automatically.",
    security(("bearer_auth" = []))


/// `PUT /api/v1/absences/{id}` — update an existing absence
#[utoipa::path(
    put,
    path = "/api/v1/absences/{id}",
    tag = "Absences",
    summary = "Update an absence",
    description = "Update absence_type, start_date, end_date, or notes. Only the owner or an admin may update.",
    security(("bearer_auth" = []))

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
