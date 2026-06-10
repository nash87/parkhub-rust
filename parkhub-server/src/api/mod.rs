//! HTTP API Routes
//!
//! `RESTful` API for the parking system.

use axum::{
    Extension, Json, Router,
    body::Body,
    extract::State,
    http::{HeaderName, HeaderValue, Request, StatusCode, header},
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
#[cfg(feature = "full")]
use utoipa::OpenApi;
#[cfg(feature = "full")]
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use crate::demo;
use crate::metrics;
#[cfg(feature = "full")]
use crate::openapi::ApiDoc;
use crate::rate_limit::{
    EndpointRateLimiters, IdentityBucketKind, IdentityRateLimiters, identity_rate_limit_middleware,
    ip_rate_limit_middleware,
};
use crate::static_files;

/// Maximum allowed request body size: 4 MiB.
/// Raised from 1 MiB to accommodate base64-encoded vehicle photos (max 2 MB raw
/// ≈ 2.7 MB base64 + JSON envelope).  Normal API payloads remain well under this.
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024; // 4 MiB

/// Maximum raw photo size in bytes (2 MB).
#[cfg(feature = "mod-vehicles")]
pub const MAX_PHOTO_BYTES: usize = 2 * 1024 * 1024;

// `api::tax::resolve_standard_rate` now owns the VAT-rate resolution. The
// historical `pub(super) const VAT_RATE = 0.19;` constant was retired when
// the multi-country tax profile layer shipped — callers should go through
// the resolver so the seller country is honoured.

use parkhub_common::{ApiResponse, LoginResponse, UserRole};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Submodules
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "mod-absence-approval")]
pub mod absence_approval;
#[cfg(feature = "mod-absences")]
pub mod absences;
#[cfg(feature = "mod-accessible")]
pub mod accessible;
pub mod admin;
#[cfg(feature = "mod-admin-analytics")]
pub mod admin_analytics;
pub mod admin_ext;
pub mod admin_handlers;
#[cfg(feature = "mod-analytics")]
pub mod analytics;
#[cfg(feature = "mod-announcements")]
pub mod announcements;
#[cfg(feature = "mod-api-docs")]
pub mod api_docs;
#[cfg(feature = "mod-audit-export")]
pub mod audit_export;
pub mod auth;
#[cfg(feature = "mod-cost-center")]
pub mod billing;
#[cfg(feature = "mod-bookings")]
pub mod bookings;
#[cfg(feature = "mod-branding")]
pub mod branding;
#[cfg(feature = "mod-calendar")]
pub mod calendar;
#[cfg(feature = "mod-calendar-drag")]
pub mod calendar_drag;
#[cfg(feature = "mod-bookings")]
pub mod co2;
#[cfg(feature = "mod-compliance")]
pub mod compliance;
#[cfg(feature = "mod-credits")]
pub mod credits;
#[cfg(feature = "mod-data-import")]
pub mod data_management;
#[cfg(feature = "mod-dynamic-pricing")]
pub mod dynamic_pricing;
#[cfg(feature = "mod-enhanced-pwa")]
pub mod enhanced_pwa;
#[cfg(feature = "mod-ev-charging")]
pub mod ev_charging;
#[cfg(feature = "mod-export")]
pub mod export;
#[cfg(feature = "mod-favorites")]
pub mod favorites;
#[cfg(feature = "mod-fleet")]
pub mod fleet;
#[cfg(feature = "mod-geofence")]
pub mod geofence;
#[cfg(feature = "mod-graphql")]
pub mod graphql;
#[cfg(feature = "mod-guest")]
pub mod guest;
#[cfg(feature = "mod-history")]
pub mod history;
#[cfg(feature = "mod-import")]
pub mod import;
#[cfg(feature = "mod-invoices")]
pub mod invoices;
#[cfg(feature = "mod-lobby-display")]
pub mod lobby;
pub mod lots;
pub mod lots_ext;
#[cfg(feature = "mod-maintenance")]
pub mod maintenance;
#[cfg(feature = "mod-map")]
pub mod map;
pub mod misc;
#[cfg(feature = "mod-mobile")]
pub mod mobile;
pub mod modules;
#[cfg(feature = "mod-notification-center")]
pub mod notification_center;
#[cfg(feature = "mod-notifications")]
#[allow(dead_code)]
pub mod notification_channels;
#[cfg(feature = "mod-notifications")]
pub mod notifications;
#[cfg(feature = "mod-oauth")]
pub mod oauth;
#[cfg(feature = "mod-operating-hours")]
pub mod operating_hours;
#[cfg(feature = "mod-parking-pass")]
pub mod parking_pass;
#[cfg(feature = "mod-parking-zones")]
pub mod parking_zones;
#[cfg(feature = "mod-payments")]
pub mod payments;
#[cfg(feature = "mod-plugins")]
#[allow(dead_code)]
pub mod plugins;
#[cfg(feature = "mod-push")]
#[allow(dead_code)]
pub mod push;
// mod-pwa retired: manifest.json and sw.js are served by the static
// file handler from parkhub-web/dist/*, built from the Astro source.
// The dynamic Rust manifest was narrower (7 fields) than the Astro one
// (17 fields incl. screenshots, shortcuts, categories, lang, dir) and
// shadowed the richer PWA install experience.
#[cfg(feature = "mod-qr")]
pub mod qr;
pub mod rate_dashboard;
#[cfg(feature = "mod-rbac")]
pub mod rbac;
#[cfg(feature = "mod-recommendations")]
pub mod recommendation_allocation;
#[cfg(feature = "mod-recommendations")]
pub mod recommendations;
#[cfg(feature = "mod-recurring")]
pub mod recurring;
pub mod retention;
#[cfg(feature = "mod-scheduled-reports")]
pub mod scheduled_reports;
pub mod security;
#[cfg(feature = "mod-settings")]
pub mod settings;
pub mod setup;
#[cfg(feature = "mod-sharing")]
pub mod sharing;
#[cfg(test)]
mod snapshots;
#[cfg(feature = "mod-social")]
mod social;
/// T-1946 — Server-Sent Events for realtime fleet updates.
pub mod sse;
#[cfg(feature = "mod-sso")]
pub mod sso;
#[cfg(feature = "mod-stripe")]
pub mod stripe;
#[cfg(feature = "mod-swap")]
pub mod swap;
pub mod system;
pub mod tax;
#[cfg(feature = "mod-team")]
pub mod team;
#[cfg(feature = "mod-multi-tenant")]
pub mod tenants;
#[cfg(feature = "mod-translations")]
pub mod translations;
pub mod updates;
pub mod users;
#[cfg(feature = "mod-vehicles")]
pub mod vehicles;
#[cfg(feature = "mod-api-versioning")]
pub mod versioning;
#[cfg(feature = "mod-visitors")]
pub mod visitors;
#[cfg(feature = "mod-waitlist")]
pub mod waitlist;
#[cfg(feature = "mod-waitlist-ext")]
pub mod waitlist_ext;
#[cfg(feature = "mod-webhooks")]
pub mod webhooks;
#[cfg(feature = "mod-webhooks-v2")]
pub mod webhooks_v2;
#[cfg(feature = "mod-widgets")]
pub mod widgets;
pub mod ws;
#[cfg(feature = "mod-zones")]
pub mod zones;

// Re-import handler functions so the router can reference them unqualified.
#[cfg(feature = "mod-absence-approval")]
use absence_approval::{
    approve_absence, list_pending_absences, my_absence_requests, reject_absence,
    submit_absence_request,
};
#[cfg(feature = "mod-absences")]
use absences::{
    create_absence, delete_absence, get_absence_pattern, list_absences, list_team_absences,
    save_absence_pattern, update_absence,
};
#[cfg(feature = "mod-announcements")]
use announcements::{
    admin_create_announcement, admin_delete_announcement, admin_list_announcements,
    admin_update_announcement, get_active_announcements,
};
use auth::{
    forgot_password, login, login_alias, logout, refresh_token, refresh_token_alias, register,
    register_alias, reset_password,
};
#[cfg(feature = "mod-bookings")]
pub use bookings::{
    booking_checkin, cancel_booking, create_booking, get_booking, get_booking_invoice,
    list_bookings, quick_book, update_booking,
};
#[cfg(feature = "mod-calendar")]
use calendar::{
    calendar_events, calendar_ical_authenticated, calendar_ical_by_token, generate_calendar_token,
    user_calendar_ics,
};
#[cfg(feature = "mod-calendar-drag")]
use calendar_drag::reschedule_booking;
#[cfg(feature = "mod-credits")]
use credits::{
    admin_grant_credits, admin_list_credit_transactions, admin_refill_all_credits,
    admin_update_user_quota, get_user_credits,
};
#[cfg(feature = "mod-ev-charging")]
use ev_charging::{
    admin_add_charger, admin_charger_overview, charging_history, list_lot_chargers, start_charging,
    stop_charging,
};
#[cfg(feature = "mod-export")]
use export::{admin_export_bookings_csv, admin_export_revenue_csv, admin_export_users_csv};
#[cfg(feature = "mod-favorites")]
use favorites::{add_favorite, list_favorites, remove_favorite};
#[cfg(feature = "mod-geofence")]
use geofence::{admin_set_geofence, geofence_check_in, get_lot_geofence};
#[cfg(feature = "mod-guest")]
use guest::{
    admin_cancel_guest_booking, admin_list_guest_bookings, create_guest_booking,
    list_user_guest_bookings,
};
#[cfg(feature = "mod-history")]
use history::{booking_history, booking_stats};
#[cfg(feature = "mod-import")]
use import::import_users_csv;
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_pricing, get_lot_slots,
    list_lots, update_lot, update_lot_pricing, update_slot,
};
#[cfg(feature = "mod-mobile")]
use mobile::{active_booking, nearby_lots, quick_book as mobile_quick_book};
#[cfg(feature = "mod-notification-center")]
use notification_center::{
    delete_notification, list_center_notifications, mark_all_read, unread_count,
};
#[cfg(feature = "mod-notifications")]
use notifications::{list_notifications, mark_all_notifications_read, mark_notification_read};
#[cfg(feature = "mod-recommendations")]
use recommendation_allocation::solve_exact_cover_allocation;
#[cfg(feature = "mod-recommendations")]
use recommendations::{get_recommendation_stats, get_recommendations};
#[cfg(feature = "mod-recurring")]
use recurring::{
    create_recurring_booking, delete_recurring_booking, list_recurring_bookings,
    update_recurring_booking,
};
#[cfg(feature = "mod-settings")]
use settings::{
    admin_get_features, admin_get_settings, admin_get_use_case, admin_update_features,
    admin_update_settings, get_features, get_public_theme,
};
// Re-export read_admin_setting from settings module when available,
// otherwise provide inline fallback (used by core handlers like auto-release).
#[cfg(feature = "mod-settings")]
use settings::read_admin_setting;
#[cfg(not(feature = "mod-settings"))]
async fn read_admin_setting(db: &crate::db::Database, key: &str) -> String {
    const DEFAULTS: &[(&str, &str)] = &[
        ("auto_release_enabled", "true"),
        ("auto_release_minutes", "30"),
        ("require_vehicle", "false"),
        ("waitlist_enabled", "true"),
        ("min_booking_duration_hours", "0"),
        ("max_booking_duration_hours", "0"),
        ("credits_enabled", "false"),
        ("credits_per_booking", "1"),
    ];
    if let Ok(Some(val)) = db.get_setting(key).await {
        return val;
    }
    DEFAULTS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
        .unwrap_or_default()
}
#[cfg(feature = "mod-parking-pass")]
use parking_pass::{get_booking_pass, list_my_passes, verify_pass};
#[cfg(feature = "mod-rbac")]
use rbac::{assign_user_roles, create_role, delete_role, get_user_roles, list_roles, update_role};
#[cfg(feature = "mod-sso")]
use sso::{
    sso_callback, sso_configure_provider, sso_delete_provider, sso_list_providers, sso_login,
};
#[cfg(feature = "mod-swap")]
use swap::{create_swap_request, list_swap_requests, update_swap_request};
#[cfg(feature = "mod-team")]
use team::{team_list, team_today};
#[cfg(feature = "mod-translations")]
use translations::{
    create_proposal, get_proposal, list_overrides, list_proposals, review_proposal,
    vote_on_proposal,
};
#[cfg(feature = "mod-vehicles")]
use vehicles::{
    create_vehicle, delete_vehicle, get_vehicle_photo, list_vehicles, update_vehicle,
    upload_vehicle_photo, vehicle_city_codes,
};
#[cfg(feature = "mod-visitors")]
use visitors::{
    admin_list_visitors, cancel_visitor, check_in_visitor, list_my_visitors, register_visitor,
};
#[cfg(feature = "mod-waitlist")]
use waitlist::{join_waitlist, leave_waitlist, list_waitlist};
#[cfg(feature = "mod-waitlist-ext")]
use waitlist_ext::{
    accept_waitlist_offer, decline_waitlist_offer, get_lot_waitlist, leave_lot_waitlist,
    subscribe_waitlist,
};
#[cfg(feature = "mod-webhooks")]
use webhooks::{create_webhook, delete_webhook, list_webhooks, test_webhook, update_webhook};
#[cfg(feature = "mod-widgets")]
use widgets::{get_widget_data, get_widget_layout, save_widget_layout};
#[cfg(feature = "mod-zones")]
use zones::{create_zone, delete_zone, list_zones, update_zone};

// Re-exports from extracted modules (Phase 3)
pub use admin_handlers::{
    admin_audit_log, admin_audit_log_export, admin_delete_user, admin_get_auto_release,
    admin_get_email_settings, admin_get_privacy, admin_heatmap, admin_list_bookings,
    admin_list_users, admin_reports, admin_reset, admin_stats, admin_update_auto_release,
    admin_update_email_settings, admin_update_privacy, admin_update_user, admin_update_user_role,
    admin_update_user_status,
};
pub use lots_ext::{admin_dashboard_charts, lot_qr_code};
pub use misc::{
    get_impressum, get_impressum_admin, public_display, public_occupancy, update_impressum,
};
pub use users::{
    auth_change_password, change_password, gdpr_delete_account, gdpr_export_data, get_current_user,
    get_my_settings, get_user, get_user_preferences, update_current_user, update_my_settings,
    update_user_preferences, user_stats,
};

/// User ID extracted from auth token.
///
/// When the request authenticated via an API key, `api_key_id` is set so
/// downstream middleware (notably the per-identity rate limiter introduced
/// in T-1743) can use the key id as the rate-limit bucket instead of the
/// user id, giving each key its own quota.
#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    /// API key id when the request authenticated via `X-API-Key` header.
    /// `None` for session/bearer/cookie auth.
    pub api_key_id: Option<Uuid>,
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

/// T-1731: resolve the caller's `tenant_id` by looking up the authenticated user.
///
/// Returns the user's `tenant_id` field (which is `None` for platform admins
/// or unbound accounts — the same semantic the PHP side uses via
/// `TenantScope::currentId()`).  Returns `None` when the user cannot be
/// loaded at all (caller should treat that as "no tenant bound" rather than
/// failing the request — the auth layer already guaranteed a valid `user_id`).
///
/// This is the first half of the Rust-side multi-tenancy hardening: every
/// domain object created on behalf of `auth_user` carries the caller's
/// `tenant_id` instead of a hard-coded `None`, so when `MODULE_MULTI_TENANT`
/// flips on (today it is OFF) the records are already partitioned correctly.
pub async fn resolve_tenant_id(state: &crate::AppState, user_id: Uuid) -> Option<String> {
    state
        .db
        .get_user(&user_id.to_string())
        .await
        .ok()
        .flatten()
        .and_then(|u| u.tenant_id)
}

/// T-1731: read-path guard — does this entity belong to the caller's tenant?
///
/// Semantics mirror the PHP `TenantScope`:
/// * caller with `tenant_id = None` (platform admin / unbound) → sees
///   everything (returns true unconditionally).  The flag-off default also
///   resolves every user to `None`, so current behaviour is preserved.
/// * caller with `tenant_id = Some(t)` → only entities with the same
///   `tenant_id` are visible.
///
/// Use this as a `.filter()` predicate on `Vec<T>` returned from bulk list
/// calls that don't yet have a tenant predicate in the DB query.
#[must_use]
pub fn matches_tenant(entity_tenant: Option<&str>, caller_tenant: Option<&str>) -> bool {
    match caller_tenant {
        None => true,
        Some(caller) => entity_tenant == Some(caller),
    }
}

/// Middleware that enforces admin role for an entire route group (issue #109).
///
/// Expects `AuthUser` to be in request extensions (set by `auth_middleware`).
/// Returns 403 FORBIDDEN if the user is not an admin or superadmin.
async fn admin_middleware(
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error("UNAUTHORIZED", "Not authenticated")),
            )
        })?;

    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return Err((status, Json(ApiResponse::error("FORBIDDEN", msg))));
    }
    drop(state_guard);

    Ok(next.run(request).await)
}

// `GET /api/v1/modules` and `GET /api/v1/modules/{name}` live in
// `api::modules` — see the declarative registry (`ModuleDef` table) and
// `ListModulesResponse` envelope there. The legacy flat `{name: bool}`
// map is preserved under the `modules` field for backward compatibility.

// ═════════════════════════════════════════════════════════════════════════════
// ROUTE-GROUP HELPERS (T-1741)
// ═════════════════════════════════════════════════════════════════════════════
//
// `create_router` composes a Router by *merging* these focused helpers, one per
// cohesive endpoint group. Each helper returns a `Router<SharedState>` with its
// route-local extensions/layers already applied (e.g. the per-route rate limits
// on the pre-auth `/auth/*` sub-routers, or the `Extension(payment_store)` on
// the payments group). Cross-cutting middleware — `auth_middleware`,
// `admin_middleware`, the outer tower stack — is still layered in
// `create_router` itself so the order matches the pre-split implementation
// verbatim. No handler's visibility or request-processing order should change.
//
// Helpers are `#[allow(unused_variables, clippy::too_many_lines)]` because
// feature-gated blocks may elide every use of a rate-limiter arg in a given
// build (e.g. `--no-default-features`), and the booking/integration helpers
// intentionally group many small route blocks by domain.

/// Rate-limited, pre-auth auth routes and aliases:
/// `/api/v1/auth/*`, legacy `/api/v1/{login,register,refresh}`, and the
/// standalone `/api/v1/bookings/{id}/qr` route.
///
/// Each sub-router has its own per-IP + per-identity rate-limit layers applied
/// via `route_layer` (so only that route is affected), and `two_fa_store` is
/// installed as an extension on the login + 2FA-login routes only.
fn auth_rate_limited_routes(
    rate_limiters: &EndpointRateLimiters,
    identity_limiters: &Arc<IdentityRateLimiters>,
    two_fa_store: Arc<security::TwoFactorTempTokenStore>,
) -> Router<SharedState> {
    // POST /api/v1/auth/login — 5/min per IP, Login bucket per identity
    let login_limiter = rate_limiters.login.clone();
    let login_identity = identity_limiters.clone();
    let login_route = Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/login", post(login_alias))
        .layer(Extension(two_fa_store.clone()))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                login_identity.clone(),
                IdentityBucketKind::Login,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(login_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/2fa/login — 5/min per IP, Login bucket per identity
    let two_fa_login_limiter = rate_limiters.login.clone();
    let two_fa_login_identity = identity_limiters.clone();
    let two_fa_login_route = Router::new()
        .route("/api/v1/auth/2fa/login", post(security::two_factor_login))
        .layer(Extension(two_fa_store))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                two_fa_login_identity.clone(),
                IdentityBucketKind::Login,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(two_fa_login_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/register — 3/min per IP, Register bucket per identity
    let register_limiter = rate_limiters.register.clone();
    let register_identity = identity_limiters.clone();
    let register_route = Router::new()
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/register", post(register_alias))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                register_identity.clone(),
                IdentityBucketKind::Register,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(register_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/forgot-password — 3/15 min per IP, PasswordReset bucket
    let forgot_limiter = rate_limiters.forgot_password.clone();
    let forgot_identity = identity_limiters.clone();
    let forgot_route = Router::new()
        .route("/api/v1/auth/forgot-password", post(forgot_password))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                forgot_identity.clone(),
                IdentityBucketKind::PasswordReset,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(forgot_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/refresh — 10/min per IP, Mutation bucket per identity
    let refresh_limiter = rate_limiters.token_refresh.clone();
    let refresh_identity = identity_limiters.clone();
    let refresh_route = Router::new()
        .route("/api/v1/auth/refresh", post(refresh_token))
        .route("/api/v1/refresh", post(refresh_token_alias))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                refresh_identity.clone(),
                IdentityBucketKind::Mutation,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(refresh_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/reset-password — 5/15 min per IP, PasswordReset bucket
    let reset_pw_limiter = rate_limiters.password_reset.clone();
    let reset_pw_identity = identity_limiters.clone();
    let reset_password_route = Router::new()
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route_layer(middleware::from_fn(move |req, next| {
            identity_rate_limit_middleware(
                reset_pw_identity.clone(),
                IdentityBucketKind::PasswordReset,
                req,
                next,
            )
        }))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(reset_pw_limiter.clone(), req, next)
        }));

    // POST /api/v1/auth/logout — clears httpOnly cookie, invalidates session
    let logout_route = Router::new().route("/api/v1/auth/logout", post(logout));

    Router::new()
        .merge(login_route)
        .merge(two_fa_login_route)
        .merge(register_route)
        .merge(forgot_route)
        .merge(refresh_route)
        .merge(reset_password_route)
        .merge(logout_route)
}

/// Standalone QR pass route — 10/min per IP, auth-middleware applied inside
/// the sub-router (not by the outer protected-routes auth layer).
#[cfg(feature = "mod-qr")]
fn qr_pass_route(state: SharedState, rate_limiters: &EndpointRateLimiters) -> Router<SharedState> {
    let qr_limiter = rate_limiters.qr_pass.clone();
    Router::new()
        .route("/api/v1/bookings/{id}/qr", get(qr::booking_qr_code))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
        .route_layer(middleware::from_fn(move |req, next| {
            ip_rate_limit_middleware(qr_limiter.clone(), req, next)
        }))
}

/// Public (unauthenticated) routes: health, setup, module registry, legal,
/// WebSocket handshake, and all feature-gated public surfaces (docs, graphql
/// playground, pass verification, shared booking view, version + changelog,
/// audit-export download, calendar iCal by token, feature flags + theme,
/// active announcements, VAPID key, map markers, Stripe webhook + config,
/// PWA manifest, branding logo, SSO + OAuth entry points, lobby display).
#[allow(unused_variables, clippy::too_many_lines)]
fn public_routes(state: &SharedState, rate_limiters: &EndpointRateLimiters) -> Router<SharedState> {
    let mut router = Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/detailed", get(admin_ext::detailed_health_check))
        .route("/api/v1/health", get(v1_health))
        .route("/api/v1/health/live", get(v1_health_live))
        .route("/api/v1/health/ready", get(v1_health_ready))
        .route("/api/v1/health/info", get(v1_health_info))
        .route("/api/v1/health/detailed", get(v1_health_detailed))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        .route("/api/v1/status", get(v1_server_status))
        .route("/api/v1/discover", get(v1_discover))
        // Legal — public (DDG § 5 requires Impressum to be freely accessible)
        .route("/api/v1/legal/impressum", get(get_impressum))
        // Module registry — public (compile-time feature introspection
        // plus category/description/config-keys/UI-deep-links/dependencies
        // for the admin Modules Dashboard and the Command Palette).
        .route("/api/v1/modules", get(modules::list_modules))
        .route("/api/v1/modules/{name}", get(modules::get_module))
        // Legacy aliases — the enriched endpoint used to live at
        // `/api/v1/modules/info`. Kept so older frontends keep working.
        .route("/api/v1/modules/info", get(modules::list_modules))
        .route("/api/v1/modules/info/{name}", get(modules::get_module))
        // Setup wizard — only works before initial setup is completed
        .route("/api/v1/setup/status", get(setup::setup_status))
        .route("/api/v1/setup", post(setup::setup_init))
        // Public occupancy display (no auth)
        .route("/api/v1/public/occupancy", get(public_occupancy))
        .route("/api/v1/public/display", get(public_display))
        // System info (public — no auth needed for version/maintenance checks)
        .route("/api/v1/system/version", get(system_version))
        .route("/api/v1/system/maintenance", get(system_maintenance));

    #[cfg(feature = "mod-websocket")]
    {
        // Real-time WebSocket endpoint is part of the public contract only
        // when the websocket module is compiled in.
        router = router.route("/api/v1/ws", get(ws::ws_handler));
    }

    // T-1946 — Server-Sent Events for fleet screens (Einchecken/EV/Tausch).
    // Auth is performed inside the handler (cookie OR bearer) because
    // `auth_middleware` enforces an `X-Requested-With` CSRF header that
    // browser `EventSource` cannot set.
    router = router.route("/api/v1/events/fleet", get(sse::fleet_events_handler));

    // Setup wizard (multi-step onboarding) — public for initial setup
    #[cfg(feature = "mod-setup-wizard")]
    {
        router = router
            .route("/api/v1/setup/wizard/status", get(system::wizard_status))
            .route("/api/v1/setup/wizard", post(system::wizard_step));
    }

    // Lobby display — rate-limited public route (10 req/min per IP)
    #[cfg(feature = "mod-lobby-display")]
    {
        let lobby_limiter = rate_limiters.lobby_display.clone();
        let lobby_route = Router::new()
            .route("/api/v1/lots/{id}/display", get(lobby::lot_display))
            .route_layer(middleware::from_fn(move |req, next| {
                ip_rate_limit_middleware(lobby_limiter.clone(), req, next)
            }))
            .with_state(state.clone());
        router = router.merge(lobby_route);
    }

    // Interactive API documentation (public)
    #[cfg(feature = "mod-api-docs")]
    {
        router = router
            .route("/api/v1/docs", get(api_docs::api_docs_ui))
            .route(
                "/api/v1/docs/openapi.json",
                get(api_docs::api_docs_openapi_json),
            )
            .route(
                "/api/v1/docs/postman.json",
                get(api_docs::api_docs_postman_json),
            );
    }

    // GraphQL playground (public, no auth — playground UI)
    #[cfg(feature = "mod-graphql")]
    {
        router = router
            .route(
                "/api/v1/graphql/playground",
                get(graphql::graphql_playground),
            )
            .route("/api/v1/graphql/schema", get(graphql::graphql_schema));
    }

    // Public pass verification (no auth needed — used by QR scan)
    #[cfg(feature = "mod-parking-pass")]
    {
        router = router.route("/api/v1/pass/verify/{code}", get(verify_pass));
    }

    // Shared booking view (public, no auth — accessed via share link)
    #[cfg(feature = "mod-sharing")]
    {
        router = router.route("/api/v1/shared/{code}", get(sharing::get_shared_booking));
    }

    // API versioning (public, no auth — version and changelog info)
    #[cfg(feature = "mod-api-versioning")]
    {
        router = router
            .route("/api/v1/version", get(versioning::api_version))
            .route("/api/v1/changelog", get(versioning::api_changelog));
    }

    // Audit export download — token-based auth (no bearer needed)
    #[cfg(feature = "mod-audit-export")]
    {
        router = router.route(
            "/api/v1/admin/audit-log/export/download/{token}",
            get(audit_export::download_audit_export),
        );
    }

    // Calendar iCal via personal subscription token — public (token in URL)
    #[cfg(feature = "mod-calendar")]
    {
        router = router.route("/api/v1/calendar/ical/{token}", get(calendar_ical_by_token));
    }

    // Feature flags + public theme — frontend needs these before auth
    #[cfg(feature = "mod-settings")]
    {
        router = router
            .route("/api/v1/features", get(get_features))
            .route("/api/v1/theme", get(get_public_theme));
    }
    #[cfg(feature = "mod-announcements")]
    {
        router = router.route(
            "/api/v1/announcements/active",
            get(get_active_announcements),
        );
    }
    #[cfg(feature = "mod-push")]
    {
        router = router.route("/api/v1/push/vapid-key", get(push::get_vapid_key));
    }
    #[cfg(feature = "mod-map")]
    {
        router = router.route("/api/v1/lots/map", get(map::list_lot_markers));
    }
    #[cfg(feature = "mod-stripe")]
    {
        router = router
            .route(
                "/api/v1/payments/webhook",
                post(stripe::stripe_webhook).layer(Extension(stripe::new_checkout_store())),
            )
            .route("/api/v1/payments/config", get(stripe::stripe_config));
    }
    #[cfg(feature = "mod-enhanced-pwa")]
    {
        // Enhanced PWA: dynamic manifest with branding + offline booking data.
        router = router.route(
            "/api/v1/pwa/manifest",
            get(enhanced_pwa::pwa_dynamic_manifest),
        );
    }
    #[cfg(feature = "mod-branding")]
    {
        router = router.route("/api/v1/branding/logo", get(branding::get_branding_logo));
    }
    #[cfg(feature = "mod-sso")]
    {
        router = router
            .route("/api/v1/auth/sso/providers", get(sso_list_providers))
            .route("/api/v1/auth/sso/{provider}/login", get(sso_login))
            .route("/api/v1/auth/sso/{provider}/callback", post(sso_callback));
    }
    #[cfg(feature = "mod-oauth")]
    {
        router = router
            .route("/api/v1/auth/oauth/providers", get(oauth::oauth_providers))
            .route(
                "/api/v1/auth/oauth/google",
                get(oauth::oauth_google_redirect),
            )
            .route(
                "/api/v1/auth/oauth/google/callback",
                get(oauth::oauth_google_callback),
            )
            .route(
                "/api/v1/auth/oauth/github",
                get(oauth::oauth_github_redirect),
            )
            .route(
                "/api/v1/auth/oauth/github/callback",
                get(oauth::oauth_github_callback),
            );
    }

    router
}

/// Core user routes: `/users/me`, `/me` alias, password change, GDPR
/// export/delete, admin user lookup, plus the user stats + preferences,
/// security (2FA / sessions / API keys / login history), and
/// notification/theme preference endpoints.
fn user_core_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/v1/users/me",
            get(get_current_user).put(update_current_user),
        )
        // Alias: frontend may call /api/v1/me — keep both paths working
        .route("/api/v1/me", get(get_current_user).put(update_current_user))
        // v5 customization: opaque per-user settings JSON (theme, sidebar
        // variant, density, font, feature toggles, notifications, privacy).
        .route(
            "/api/v1/me/settings",
            get(get_my_settings).put(update_my_settings),
        )
        .route("/api/v1/users/me/export", get(gdpr_export_data))
        .route("/api/v1/users/me/delete", delete(gdpr_delete_account))
        .route(
            "/api/v1/users/me/password",
            axum::routing::patch(change_password),
        )
        .route(
            "/api/v1/auth/change-password",
            axum::routing::patch(auth_change_password),
        )
        // Admin-only: retrieve any user by ID
        .route("/api/v1/users/{id}", get(get_user))
}

/// Core lot + slot CRUD, per-lot pricing, dynamic pricing (read), operating
/// hours (read), lot QR code, plus user stats and preference endpoints.
#[allow(unused_mut)]
fn lot_core_routes() -> Router<SharedState> {
    let mut router = Router::new()
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
        );

    // Dynamic pricing (occupancy-based) — user-facing read endpoint
    #[cfg(feature = "mod-dynamic-pricing")]
    {
        router = router.route(
            "/api/v1/lots/{id}/pricing/dynamic",
            get(dynamic_pricing::get_dynamic_pricing),
        );
    }

    // Operating hours — user-facing read endpoint
    #[cfg(feature = "mod-operating-hours")]
    {
        router = router.route(
            "/api/v1/lots/{id}/hours",
            get(operating_hours::get_operating_hours),
        );
    }

    router
        // QR code for lot
        .route("/api/v1/lots/{id}/qr", get(lot_qr_code))
        // User stats & preferences
        .route("/api/v1/user/stats", get(user_stats))
        .route(
            "/api/v1/user/preferences",
            get(get_user_preferences).put(update_user_preferences),
        )
}

/// Per-user security + preference routes: 2FA lifecycle, login history,
/// session management, API keys, notification preferences, design theme.
fn user_security_routes() -> Router<SharedState> {
    Router::new()
        // ── Security: 2FA ──
        .route("/api/v1/auth/2fa/setup", post(security::two_factor_setup))
        .route("/api/v1/auth/2fa/verify", post(security::two_factor_verify))
        .route("/api/v1/auth/2fa/disable", post(security::two_factor_disable))
        .route("/api/v1/auth/2fa/status", get(security::two_factor_status))
        // ── Security: Login history ──
        .route("/api/v1/auth/login-history", get(security::get_login_history))
        // ── Security: Session management ──
        .route("/api/v1/auth/sessions", get(security::list_sessions))
        .route("/api/v1/auth/sessions/{id}", delete(security::revoke_session))
        // ── Security: API keys ──
        .route(
            "/api/v1/auth/api-keys",
            get(security::list_api_keys).post(security::create_api_key),
        )
        .route("/api/v1/auth/api-keys/{id}", delete(security::revoke_api_key))
        // ── Notification preferences ──
        .route(
            "/api/v1/preferences/notifications",
            get(admin_ext::get_notification_preferences)
                .put(admin_ext::update_notification_preferences),
        )
        // ── Design theme preferences ──
        .route(
            "/api/v1/preferences/theme",
            get(admin_ext::get_design_theme_preference)
                .put(admin_ext::update_design_theme_preference),
        )
}

/// Admin sub-router — every `/api/v1/admin/*` route that is gated by the
/// shared `admin_middleware` layer (issue #109). Returns the router *without*
/// the middleware layer applied; `create_router` wraps the merged router in
/// `from_fn_with_state(state, admin_middleware)` before merging it into the
/// protected surface, matching the pre-split middleware order verbatim.
#[allow(unused_mut, clippy::too_many_lines)]
fn admin_core_routes() -> Router<SharedState> {
    let mut admin_routes = Router::new()
        .route(
            "/api/v1/admin/impressum",
            get(get_impressum_admin).put(update_impressum),
        )
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
        .route("/api/v1/admin/bookings", get(admin_list_bookings))
        .route("/api/v1/admin/stats", get(admin_stats))
        .route("/api/v1/admin/reports", get(admin_reports))
        .route("/api/v1/admin/heatmap", get(admin_heatmap))
        .route(
            "/api/v1/admin/dashboard/charts",
            get(admin_dashboard_charts),
        )
        .route("/api/v1/admin/audit-log", get(admin_audit_log))
        .route(
            "/api/v1/admin/audit-log/export",
            get(admin_audit_log_export),
        );

    #[cfg(feature = "mod-audit-export")]
    {
        admin_routes = admin_routes.route(
            "/api/v1/admin/audit-log/export/enhanced",
            get(audit_export::enhanced_audit_export),
        );
    }

    #[cfg(feature = "mod-analytics")]
    {
        admin_routes = admin_routes.route(
            "/api/v1/admin/analytics/overview",
            get(analytics::analytics_overview),
        );
    }

    #[cfg(feature = "mod-admin-analytics")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/analytics/occupancy",
                get(admin_analytics::admin_occupancy),
            )
            .route(
                "/api/v1/admin/analytics/revenue",
                get(admin_analytics::admin_revenue_summary),
            )
            .route(
                "/api/v1/admin/analytics/popular-lots",
                get(admin_analytics::admin_popular_lots),
            );
    }

    admin_routes = admin_routes
        .route("/api/v1/admin/reset", post(admin_reset))
        .route(
            "/api/v1/admin/settings/auto-release",
            get(admin_get_auto_release).put(admin_update_auto_release),
        )
        .route(
            "/api/v1/admin/settings/email",
            get(admin_get_email_settings).put(admin_update_email_settings),
        )
        .route(
            "/api/v1/admin/privacy",
            get(admin_get_privacy).put(admin_update_privacy),
        )
        .route("/api/v1/admin/users/{id}/update", put(admin_update_user))
        .route(
            "/api/v1/admin/users/{id}/reset-password",
            post(admin_handlers::admin_reset_user_password),
        )
        // ── Security: Admin password policy ──
        .route(
            "/api/v1/admin/settings/password-policy",
            get(security::get_password_policy).put(security::update_password_policy),
        )
        // ── Security: Admin login history ──
        .route(
            "/api/v1/admin/users/{id}/login-history",
            get(security::admin_get_login_history),
        )
        // ── Bulk admin operations ──
        .route(
            "/api/v1/admin/users/bulk-update",
            post(admin_ext::bulk_update_users),
        )
        .route(
            "/api/v1/admin/users/bulk-delete",
            post(admin_ext::bulk_delete_users),
        )
        // ── Advanced reports ──
        .route(
            "/api/v1/admin/reports/revenue",
            get(admin_ext::revenue_report),
        )
        .route(
            "/api/v1/admin/reports/occupancy",
            get(admin_ext::occupancy_report),
        )
        .route(
            "/api/v1/admin/reports/users",
            get(admin_ext::user_report),
        )
        // ── Booking policies ──
        .route(
            "/api/v1/admin/settings/booking-policies",
            get(admin_ext::get_booking_policies)
                .put(admin_ext::update_booking_policies),
        )
        // ── Rate limit dashboard ──
        .route(
            "/api/v1/admin/rate-limits",
            get(rate_dashboard::admin_rate_limit_stats),
        )
        .route(
            "/api/v1/admin/rate-limits/history",
            get(rate_dashboard::admin_rate_limit_history),
        );

    #[cfg(feature = "mod-multi-tenant")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/tenants",
                get(tenants::list_tenants).post(tenants::create_tenant),
            )
            .route("/api/v1/admin/tenants/{id}", put(tenants::update_tenant));
    }

    #[cfg(feature = "mod-geofence")]
    {
        admin_routes =
            admin_routes.route("/api/v1/admin/lots/{id}/geofence", put(admin_set_geofence));
    }

    #[cfg(feature = "mod-widgets")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/widgets",
                get(get_widget_layout).put(save_widget_layout),
            )
            .route(
                "/api/v1/admin/widgets/data/{widget_id}",
                get(get_widget_data),
            );
    }

    #[cfg(feature = "mod-plugins")]
    {
        admin_routes = admin_routes
            .route("/api/v1/admin/plugins", get(plugins::list_plugins))
            .route(
                "/api/v1/admin/plugins/{id}/toggle",
                put(plugins::toggle_plugin),
            )
            .route(
                "/api/v1/admin/plugins/{id}/config",
                get(plugins::get_plugin_config).put(plugins::update_plugin_config),
            );
    }

    #[cfg(feature = "mod-compliance")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/compliance/report",
                get(compliance::compliance_report),
            )
            .route(
                "/api/v1/admin/compliance/report/pdf",
                get(compliance::compliance_report_pdf),
            )
            .route(
                "/api/v1/admin/compliance/data-map",
                get(compliance::compliance_data_map),
            )
            .route(
                "/api/v1/admin/compliance/audit-export",
                get(compliance::compliance_audit_export),
            );
    }

    #[cfg(feature = "mod-scheduled-reports")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/reports/schedules",
                get(scheduled_reports::list_schedules).post(scheduled_reports::create_schedule),
            )
            .route(
                "/api/v1/admin/reports/schedules/{id}",
                put(scheduled_reports::update_schedule).delete(scheduled_reports::delete_schedule),
            )
            .route(
                "/api/v1/admin/reports/schedules/{id}/send-now",
                post(scheduled_reports::send_now),
            );
    }

    #[cfg(feature = "mod-sso")]
    {
        admin_routes = admin_routes.route(
            "/api/v1/admin/sso/{provider}",
            put(sso_configure_provider).delete(sso_delete_provider),
        );
    }

    #[cfg(feature = "mod-webhooks-v2")]
    {
        admin_routes = admin_routes
            .route(
                "/api/v1/admin/webhooks-v2",
                get(webhooks_v2::list_webhooks_v2).post(webhooks_v2::create_webhook_v2),
            )
            .route(
                "/api/v1/admin/webhooks-v2/{id}",
                put(webhooks_v2::update_webhook_v2).delete(webhooks_v2::delete_webhook_v2),
            )
            .route(
                "/api/v1/admin/webhooks-v2/{id}/test",
                post(webhooks_v2::test_webhook_v2),
            )
            .route(
                "/api/v1/admin/webhooks-v2/{id}/deliveries",
                get(webhooks_v2::list_deliveries_v2),
            );
    }

    #[cfg(feature = "mod-rbac")]
    {
        admin_routes = admin_routes
            .route("/api/v1/admin/roles", get(list_roles).post(create_role))
            .route(
                "/api/v1/admin/roles/{id}",
                put(update_role).delete(delete_role),
            )
            .route(
                "/api/v1/admin/users/{id}/roles",
                get(get_user_roles).put(assign_user_roles),
            );
    }

    // ── Retention / GDPR deletion-policy engine ────────────────────────────
    admin_routes = admin_routes
        .route(
            "/api/v1/admin/retention/policies",
            get(retention::list_retention_policies),
        )
        .route(
            "/api/v1/admin/retention/policies/{class}",
            put(retention::update_retention_policy),
        )
        .route(
            "/api/v1/admin/retention/run",
            post(retention::run_retention),
        )
        .route(
            "/api/v1/admin/retention/evidence",
            get(retention::list_retention_evidence),
        );

    // ── Module runtime toggle — PATCH /api/v1/admin/modules/{name} ──
    // Flips the `module.{name}.runtime_enabled` admin setting for a
    // runtime-toggleable module. Security-sensitive modules return 409.
    // See `parkhub-server/src/api/modules.rs` for the allow-list policy.
    admin_routes = admin_routes.route(
        "/api/v1/admin/modules/{name}",
        axum::routing::patch(modules::patch_admin_module),
    );

    // ── Per-module JSON Schema config editor (T-1720 v3) ──────────
    // GET returns {schema, values}; PATCH validates + persists values
    // under `module.{name}.config.{field}`. Admin-gated by the shared
    // admin_middleware layer; the handlers also call `check_admin` as
    // defense-in-depth (same pattern as `patch_admin_module`).
    admin_routes.route(
        "/api/v1/admin/modules/{name}/config",
        get(modules::get_module_config).patch(modules::patch_module_config),
    )
}

/// Booking + lot-adjacent protected routes: bookings CRUD, sharing, history,
/// geofence check-in, waitlist-ext, parking-pass, calendar drag-reschedule,
/// graphql query, invoice PDF, slot QR, zones, parking-zones pricing, smart
/// recommendations.
#[allow(unused_mut)]
fn booking_protected_routes() -> Router<SharedState> {
    let mut router = Router::new();

    #[cfg(feature = "mod-bookings")]
    {
        router = router
            .route("/api/v1/bookings", get(list_bookings).post(create_booking))
            .route(
                "/api/v1/bookings/{id}",
                get(get_booking)
                    .delete(cancel_booking)
                    .patch(update_booking),
            )
            .route("/api/v1/bookings/{id}/invoice", get(get_booking_invoice))
            .route("/api/v1/bookings/quick", post(quick_book))
            .route("/api/v1/bookings/{id}/checkin", post(booking_checkin));
    }

    #[cfg(feature = "mod-sharing")]
    {
        router = router
            .route(
                "/api/v1/bookings/{id}/share",
                post(sharing::create_share_link).delete(sharing::revoke_share_link),
            )
            .route("/api/v1/bookings/{id}/invite", post(sharing::invite_guest));
    }

    #[cfg(feature = "mod-history")]
    {
        router = router
            .route("/api/v1/bookings/history", get(booking_history))
            .route("/api/v1/bookings/stats", get(booking_stats))
            .route("/api/v1/bookings/co2-summary", get(co2::co2_summary));
    }

    #[cfg(feature = "mod-geofence")]
    {
        router = router
            .route("/api/v1/geofence/check-in", post(geofence_check_in))
            .route("/api/v1/lots/{id}/geofence", get(get_lot_geofence));
    }

    #[cfg(feature = "mod-waitlist-ext")]
    {
        router = router
            .route(
                "/api/v1/lots/{id}/waitlist/subscribe",
                post(subscribe_waitlist),
            )
            .route(
                "/api/v1/lots/{id}/waitlist",
                get(get_lot_waitlist).delete(leave_lot_waitlist),
            )
            .route(
                "/api/v1/lots/{id}/waitlist/{entry_id}/accept",
                post(accept_waitlist_offer),
            )
            .route(
                "/api/v1/lots/{id}/waitlist/{entry_id}/decline",
                post(decline_waitlist_offer),
            );
    }

    #[cfg(feature = "mod-parking-pass")]
    {
        router = router
            .route("/api/v1/bookings/{id}/pass", get(get_booking_pass))
            .route("/api/v1/me/passes", get(list_my_passes));
    }

    #[cfg(feature = "mod-calendar-drag")]
    {
        router = router.route("/api/v1/bookings/{id}/reschedule", put(reschedule_booking));
    }

    #[cfg(feature = "mod-graphql")]
    {
        router = router.route("/api/v1/graphql", post(graphql::graphql_execute));
    }

    #[cfg(feature = "mod-invoices")]
    {
        router = router.route(
            "/api/v1/bookings/{id}/invoice/pdf",
            get(invoices::get_booking_invoice_pdf),
        );
    }

    #[cfg(feature = "mod-qr")]
    {
        router = router.route(
            "/api/v1/lots/{lot_id}/slots/{slot_id}/qr",
            get(qr::slot_qr_code),
        );
    }

    #[cfg(feature = "mod-zones")]
    {
        router = router
            .route(
                "/api/v1/lots/{lot_id}/zones",
                get(list_zones).post(create_zone),
            )
            .route(
                "/api/v1/lots/{lot_id}/zones/{zone_id}",
                put(update_zone).delete(delete_zone),
            );
    }

    #[cfg(feature = "mod-parking-zones")]
    {
        router = router
            .route(
                "/api/v1/lots/{id}/zones/pricing",
                get(parking_zones::list_zones_pricing),
            )
            .route(
                "/api/v1/zones/{id}/price",
                get(parking_zones::get_zone_price),
            )
            .route(
                "/api/v1/admin/zones/{id}/pricing",
                put(parking_zones::set_zone_pricing),
            );
    }

    #[cfg(feature = "mod-recommendations")]
    {
        router = router
            .route("/api/v1/bookings/recommendations", get(get_recommendations))
            .route(
                "/api/v1/recommendations/allocation/exact-cover",
                post(solve_exact_cover_allocation),
            )
            .route(
                "/api/v1/recommendations/stats",
                get(get_recommendation_stats),
            );
    }

    router
}

/// Vehicle routes: list/create, city-codes lookup, update/delete, photo
/// upload/fetch. `city-codes` is registered before `{id}` to avoid axum's
/// parameter capture swallowing the literal path.
#[allow(unused_mut)]
fn vehicle_routes() -> Router<SharedState> {
    #[allow(unused_mut)]
    let mut router = Router::new();

    #[cfg(feature = "mod-vehicles")]
    {
        router = router
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
            );
    }

    router
}

/// Credits, favorites, and settings/feature-flag admin endpoints. Groups
/// balance+admin-quota, favorite CRUD, feature flag + system-settings admin
/// surface, dynamic-pricing admin rules, operating-hours admin, import/export
/// and data-management admin endpoints, plus the fleet admin surface.
#[allow(unused_mut, clippy::too_many_lines)]
fn settings_and_data_routes() -> Router<SharedState> {
    let mut router = Router::new();

    #[cfg(feature = "mod-credits")]
    {
        router = router
            .route("/api/v1/user/credits", get(get_user_credits))
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
            );
    }

    #[cfg(feature = "mod-favorites")]
    {
        router = router
            .route(
                "/api/v1/user/favorites",
                get(list_favorites).post(add_favorite),
            )
            .route("/api/v1/user/favorites/{slot_id}", delete(remove_favorite));
    }

    #[cfg(feature = "mod-settings")]
    {
        router = router
            .route(
                "/api/v1/admin/features",
                get(admin_get_features).put(admin_update_features),
            )
            .route(
                "/api/v1/admin/settings",
                get(admin_get_settings).put(admin_update_settings),
            )
            .route("/api/v1/admin/settings/use-case", get(admin_get_use_case));
    }

    #[cfg(feature = "mod-dynamic-pricing")]
    {
        router = router.route(
            "/api/v1/admin/lots/{id}/pricing/dynamic",
            get(dynamic_pricing::admin_get_dynamic_pricing_rules)
                .put(dynamic_pricing::admin_update_dynamic_pricing_rules),
        );
    }

    #[cfg(feature = "mod-operating-hours")]
    {
        router = router.route(
            "/api/v1/admin/lots/{id}/hours",
            put(operating_hours::admin_update_operating_hours),
        );
    }

    #[cfg(feature = "mod-import")]
    {
        router = router.route("/api/v1/admin/users/import", post(import_users_csv));
    }

    #[cfg(feature = "mod-export")]
    {
        router = router
            .route("/api/v1/admin/export/users", get(admin_export_users_csv))
            .route(
                "/api/v1/admin/export/bookings",
                get(admin_export_bookings_csv),
            )
            .route(
                "/api/v1/admin/export/revenue",
                get(admin_export_revenue_csv),
            );
    }

    #[cfg(feature = "mod-data-import")]
    {
        router = router
            .route(
                "/api/v1/admin/import/users",
                post(data_management::import_users),
            )
            .route(
                "/api/v1/admin/import/lots",
                post(data_management::import_lots),
            )
            .route(
                "/api/v1/admin/data/export/users",
                get(data_management::export_users_csv),
            )
            .route(
                "/api/v1/admin/data/export/lots",
                get(data_management::export_lots_csv),
            )
            .route(
                "/api/v1/admin/data/export/bookings",
                get(data_management::export_bookings_csv),
            );
    }

    #[cfg(feature = "mod-fleet")]
    {
        router = router
            .route("/api/v1/admin/fleet", get(fleet::admin_fleet_list))
            .route("/api/v1/admin/fleet/stats", get(fleet::admin_fleet_stats))
            .route(
                "/api/v1/admin/fleet/{id}/flag",
                put(fleet::admin_fleet_flag),
            );
    }

    router
}

/// Self-update system — always available for admin (not feature-gated).
fn updates_routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/v1/admin/updates/check",
            get(updates::check_for_updates),
        )
        .route("/api/v1/admin/updates/apply", post(updates::apply_update))
        .route(
            "/api/v1/admin/updates/history",
            get(updates::update_history),
        )
        .route(
            "/api/v1/admin/updates/releases",
            get(updates::list_releases),
        )
        .route(
            "/api/v1/admin/updates/rollback",
            post(updates::rollback_update),
        )
}

/// Domain feature routes: accessible slots, maintenance, cost-center billing,
/// EV charging, absences + absence-approval + iCal import, team, announcements
/// admin, notifications, mobile quick-book, notification center, waitlist,
/// swap, recurring bookings, guest bookings, visitors, calendar, translations.
#[allow(unused_mut, clippy::too_many_lines)]
fn domain_feature_routes() -> Router<SharedState> {
    let mut router = Router::new();

    #[cfg(feature = "mod-accessible")]
    {
        router = router
            .route(
                "/api/v1/lots/{id}/slots/accessible",
                get(accessible::list_accessible_slots),
            )
            .route(
                "/api/v1/admin/lots/{id}/slots/{slot_id}/accessible",
                put(accessible::admin_set_slot_accessible),
            )
            .route(
                "/api/v1/bookings/accessible-stats",
                get(accessible::accessible_stats),
            )
            .route(
                "/api/v1/users/me/accessibility-needs",
                put(accessible::update_accessibility_needs),
            );
    }

    #[cfg(feature = "mod-maintenance")]
    {
        router = router
            .route(
                "/api/v1/admin/maintenance",
                post(maintenance::create_maintenance).get(maintenance::list_maintenance),
            )
            .route(
                "/api/v1/admin/maintenance/{id}",
                put(maintenance::update_maintenance).delete(maintenance::delete_maintenance),
            )
            .route(
                "/api/v1/maintenance/active",
                get(maintenance::active_maintenance),
            );
    }

    #[cfg(feature = "mod-cost-center")]
    {
        router = router
            .route(
                "/api/v1/admin/billing/by-cost-center",
                get(billing::billing_by_cost_center),
            )
            .route(
                "/api/v1/admin/billing/by-department",
                get(billing::billing_by_department),
            )
            .route(
                "/api/v1/admin/billing/export",
                get(billing::billing_export_csv),
            )
            .route(
                "/api/v1/admin/billing/allocate",
                post(billing::billing_allocate),
            );
    }

    #[cfg(feature = "mod-ev-charging")]
    {
        router = router
            .route("/api/v1/lots/{id}/chargers", get(list_lot_chargers))
            .route("/api/v1/chargers/{id}/start", post(start_charging))
            .route("/api/v1/chargers/{id}/stop", post(stop_charging))
            .route("/api/v1/chargers/sessions", get(charging_history))
            .route(
                "/api/v1/admin/chargers",
                get(admin_charger_overview).post(admin_add_charger),
            );
    }

    #[cfg(feature = "mod-absences")]
    {
        router = router
            .route("/api/v1/absences", get(list_absences).post(create_absence))
            .route("/api/v1/absences/team", get(list_team_absences))
            .route(
                "/api/v1/absences/pattern",
                get(get_absence_pattern).post(save_absence_pattern),
            )
            .route(
                "/api/v1/absences/{id}",
                delete(delete_absence).put(update_absence),
            );
    }

    #[cfg(feature = "mod-absence-approval")]
    {
        router = router
            .route("/api/v1/absences/requests", post(submit_absence_request))
            .route("/api/v1/absences/my", get(my_absence_requests))
            .route("/api/v1/admin/absences/pending", get(list_pending_absences))
            .route("/api/v1/admin/absences/{id}/approve", put(approve_absence))
            .route("/api/v1/admin/absences/{id}/reject", put(reject_absence));
    }

    // Absence iCal import needs both absences + import modules
    #[cfg(all(feature = "mod-absences", feature = "mod-import"))]
    {
        router = router.route(
            "/api/v1/absences/import/ical",
            post(import::import_absences_ical),
        );
    }

    #[cfg(feature = "mod-team")]
    {
        router = router
            .route("/api/v1/team/today", get(team_today))
            .route("/api/v1/team", get(team_list));
    }

    #[cfg(feature = "mod-announcements")]
    {
        router = router
            .route(
                "/api/v1/admin/announcements",
                get(admin_list_announcements).post(admin_create_announcement),
            )
            .route(
                "/api/v1/admin/announcements/{id}",
                put(admin_update_announcement).delete(admin_delete_announcement),
            );
    }

    #[cfg(feature = "mod-notifications")]
    {
        router = router
            .route("/api/v1/notifications", get(list_notifications))
            .route(
                "/api/v1/notifications/{id}/read",
                put(mark_notification_read),
            )
            .route(
                "/api/v1/notifications/read-all",
                post(mark_all_notifications_read),
            );
    }

    #[cfg(feature = "mod-mobile")]
    {
        router = router
            .route("/api/v1/mobile/quick-book", get(mobile_quick_book))
            .route("/api/v1/mobile/nearby-lots", get(nearby_lots))
            .route("/api/v1/mobile/active-booking", get(active_booking));
    }

    #[cfg(feature = "mod-notification-center")]
    {
        router = router
            .route(
                "/api/v1/notifications/center",
                get(list_center_notifications),
            )
            .route("/api/v1/notifications/unread-count", get(unread_count))
            .route("/api/v1/notifications/center/read-all", put(mark_all_read))
            .route(
                "/api/v1/notifications/center/{id}",
                delete(delete_notification),
            );
    }

    #[cfg(feature = "mod-waitlist")]
    {
        router = router
            .route("/api/v1/waitlist", get(list_waitlist).post(join_waitlist))
            .route("/api/v1/waitlist/{id}", delete(leave_waitlist));
    }

    #[cfg(feature = "mod-swap")]
    {
        router = router
            .route("/api/v1/swap-requests", get(list_swap_requests))
            .route(
                "/api/v1/bookings/{id}/swap-request",
                post(create_swap_request),
            )
            .route("/api/v1/swap-requests/{id}", put(update_swap_request));
    }

    #[cfg(feature = "mod-recurring")]
    {
        router = router
            .route(
                "/api/v1/recurring-bookings",
                get(list_recurring_bookings).post(create_recurring_booking),
            )
            .route(
                "/api/v1/recurring-bookings/{id}",
                delete(delete_recurring_booking).put(update_recurring_booking),
            );
    }

    #[cfg(feature = "mod-guest")]
    {
        // Guest bookings. The same /bookings/guest path serves GET (list the
        // current user's own passes — used by the GuestPass page on mount)
        // and POST (create a new pass). Register both on one router entry
        // or they race and the later insert wins.
        router = router
            .route(
                "/api/v1/bookings/guest",
                get(list_user_guest_bookings).post(create_guest_booking),
            )
            .route(
                "/api/v1/admin/guest-bookings",
                get(admin_list_guest_bookings),
            )
            .route(
                "/api/v1/admin/guest-bookings/{id}/cancel",
                axum::routing::patch(admin_cancel_guest_booking),
            );
    }

    #[cfg(feature = "mod-visitors")]
    {
        router = router
            .route("/api/v1/visitors/register", post(register_visitor))
            .route("/api/v1/visitors", get(list_my_visitors))
            .route("/api/v1/visitors/{id}/check-in", put(check_in_visitor))
            .route("/api/v1/visitors/{id}", delete(cancel_visitor))
            .route("/api/v1/admin/visitors", get(admin_list_visitors));
    }

    #[cfg(feature = "mod-calendar")]
    {
        router = router
            .route("/api/v1/calendar/events", get(calendar_events))
            .route("/api/v1/user/calendar.ics", get(user_calendar_ics))
            .route("/api/v1/bookings/ical", get(calendar_ical_authenticated))
            .route("/api/v1/calendar/ical", get(calendar_ical_authenticated))
            .route("/api/v1/calendar/token", post(generate_calendar_token));
    }

    #[cfg(feature = "mod-translations")]
    {
        router = router
            .route("/api/v1/translations/overrides", get(list_overrides))
            .route(
                "/api/v1/translations/proposals",
                get(list_proposals).post(create_proposal),
            )
            .route("/api/v1/translations/proposals/{id}", get(get_proposal))
            .route(
                "/api/v1/translations/proposals/{id}/vote",
                post(vote_on_proposal),
            )
            .route(
                "/api/v1/translations/proposals/{id}/review",
                put(review_proposal),
            );
    }

    router
}

/// Integration routes: webhooks, push subscribe/unsubscribe, branding admin,
/// map admin, payments (Stripe stub + checkout), and enhanced-PWA offline
/// data. Each feature group installs its own extension store inline so the
/// existing `.layer(Extension(...))` ordering is preserved verbatim.
#[allow(unused_mut)]
fn integration_routes() -> Router<SharedState> {
    let mut router = Router::new();

    #[cfg(feature = "mod-webhooks")]
    {
        router = router
            .route("/api/v1/webhooks", get(list_webhooks).post(create_webhook))
            .route(
                "/api/v1/webhooks/{id}",
                put(update_webhook).delete(delete_webhook),
            )
            .route("/api/v1/webhooks/{id}/test", post(test_webhook));
    }

    #[cfg(feature = "mod-push")]
    {
        router = router
            .route("/api/v1/push/subscribe", post(push::subscribe))
            .route("/api/v1/push/unsubscribe", delete(push::unsubscribe));
    }

    #[cfg(feature = "mod-branding")]
    {
        router = router
            .route(
                "/api/v1/admin/branding",
                get(branding::admin_get_branding).put(branding::admin_update_branding),
            )
            .route(
                "/api/v1/admin/branding/logo",
                post(branding::admin_upload_logo),
            );
    }

    #[cfg(feature = "mod-map")]
    {
        router = router.route(
            "/api/v1/admin/lots/{id}/location",
            put(map::set_lot_location),
        );
    }

    #[cfg(feature = "mod-payments")]
    {
        // Payments (Stripe stub). The Extension applies to routes added above
        // this line only — matches pre-split behaviour verbatim.
        router = router
            .route(
                "/api/v1/payments/create-intent",
                post(payments::create_payment_intent),
            )
            .route("/api/v1/payments/confirm", post(payments::confirm_payment))
            .route(
                "/api/v1/payments/{id}/status",
                get(payments::payment_status),
            )
            .layer(Extension(payments::new_payment_store()));
    }

    #[cfg(feature = "mod-stripe")]
    {
        // Stripe checkout (authenticated routes). Extension applies to routes
        // added above this line only — matches pre-split behaviour verbatim.
        let stripe_store = stripe::new_checkout_store();
        router = router
            .route(
                "/api/v1/payments/create-checkout",
                post(stripe::create_checkout),
            )
            .route("/api/v1/payments/history", get(stripe::payment_history))
            .layer(Extension(stripe_store));
    }

    #[cfg(feature = "mod-enhanced-pwa")]
    {
        router = router.route(
            "/api/v1/pwa/offline-data",
            get(enhanced_pwa::pwa_offline_data),
        );
    }

    router
}

/// Public demo-mode routes (vote + reset are rate-limited, status + config
/// are not). Returns the router plus the demo state so `create_router` can
/// hand the state to the caller for scheduled resets.
fn demo_routes(
    rate_limiters: &EndpointRateLimiters,
) -> (Router<SharedState>, demo::SharedDemoState) {
    let demo_state = demo::new_demo_state();
    let demo_state_ret = demo_state.clone();
    let demo_limiter = rate_limiters.demo.clone();
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
    let router = Router::new()
        .route("/api/v1/demo/status", get(demo::demo_status))
        .route("/api/v1/demo/config", get(demo::demo_config))
        .merge(demo_vote_route)
        .merge(demo_reset_route)
        .layer(Extension(demo_state));
    (router, demo_state_ret)
}

/// Create the API router with `OpenAPI` docs and metrics.
/// Returns (router, `demo_state`) so the demo state can be used for scheduled resets.
///
/// `revocation_store` is injected as an axum `Extension` so the `AuthUser`
/// extractor (see `crate::jwt`) can consult it on every token validation.
/// Callers in tests build one via `state.read().await.revocation_store.clone()`;
/// the production `main.rs` path passes the same `Arc` it stored in `AppState`.
#[allow(unused_mut, unused_variables)]
pub fn create_router(
    state: SharedState,
    revocation_store: Arc<crate::jwt::TokenRevocationList>,
) -> (Router, demo::SharedDemoState) {
    // ── Initialization: metrics + rate-limit infrastructure ───────────────
    let metrics_handle = metrics::init_metrics();

    let rate_limiters = EndpointRateLimiters::new();
    let global_limiter = rate_limiters.general.clone();
    let identity_limiters = rate_limiters.identity.clone();

    // Spawn the per-identity idle-entry eviction task (sweeps every 60 s).
    // Returns `None` when no tokio runtime is available — fine in unit tests
    // that sweep manually. Dropping the handle detaches the task; the process
    // terminates alongside it.
    let _identity_eviction =
        crate::rate_limit::per_identity::spawn_eviction_task(identity_limiters.clone());

    // 2FA temporary token store — shared between login and 2FA login routes
    let two_fa_store = security::TwoFactorTempTokenStore::new();

    // ── Compose route groups via helpers ──────────────────────────────────
    // Each helper returns a `Router<SharedState>` with its route-local layers
    // (per-route rate limiters, feature-gated endpoints) already applied.
    // Cross-cutting middleware (auth, admin, outer tower stack) is still
    // layered below so the order matches the pre-split implementation.
    let auth_public = auth_rate_limited_routes(&rate_limiters, &identity_limiters, two_fa_store);
    let public = public_routes(&state, &rate_limiters);
    let (demo, demo_state_ret) = demo_routes(&rate_limiters);

    // ── Protected (auth-required) routes ──────────────────────────────────
    // Build admin sub-router first, apply admin_middleware, then merge into
    // the protected surface. Order matches pre-split verbatim: admin routes
    // added via `admin_core_routes` are guarded by admin_middleware; later
    // `/api/v1/admin/*` routes (e.g. credits admin, dynamic-pricing admin)
    // added through `settings_and_data_routes` / `domain_feature_routes` are
    // intentionally NOT wrapped by admin_middleware — they rely on handler-
    // level `check_admin` only. See feedback_modular_refactor rationale.
    let admin_with_guard = admin_core_routes().route_layer(middleware::from_fn_with_state(
        state.clone(),
        admin_middleware,
    ));

    let protected_routes = Router::new()
        .merge(user_core_routes())
        .merge(lot_core_routes())
        .merge(user_security_routes())
        .merge(admin_with_guard)
        .merge(booking_protected_routes())
        .merge(vehicle_routes())
        .merge(settings_and_data_routes())
        .merge(updates_routes())
        .merge(domain_feature_routes())
        .merge(integration_routes());

    // Apply per-identity rate limiting layered INSIDE auth_middleware so the
    // layer sees the `AuthUser` that auth_middleware inserts. Bucket kind is
    // resolved at request time from method + path (Admin / Read / Mutation).
    // Login / Register / PasswordReset buckets are wired directly on the
    // pre-auth `/api/v1/auth/*` sub-routers above.
    let protected_identity_limiters = identity_limiters.clone();
    let protected_routes = protected_routes.route_layer(middleware::from_fn(move |req, next| {
        protected_identity_rate_limit_middleware(protected_identity_limiters.clone(), req, next)
    }));

    // Apply auth middleware to all protected routes. Runs OUTSIDE the
    // per-identity limiter above so `AuthUser` is present by the time the
    // identity limiter fires.
    let protected_routes = protected_routes.route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth_middleware,
    ));

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

    // ── Merge all route groups into the root router ──────────────────────
    let mut router = Router::new()
        .merge(public)
        .merge(auth_public)
        .merge(demo)
        .merge(protected_routes);

    // ── Runtime module gate (T-1720 v2) ─────────────────────────────────
    // Short-circuits requests targeting routes owned by a module that is
    // currently disabled via admin setting. Applied to the merged router
    // so both public and protected surfaces inherit the gate. Paths not
    // listed in `modules::MODULE_ROUTES` pass through unchanged — see
    // the table for the authoritative coverage list (currently: map,
    // graphql, api-docs, announcements, favorites).
    router = router.layer(middleware::from_fn_with_state(
        state.clone(),
        modules::module_gate,
    ));

    #[cfg(feature = "mod-qr")]
    {
        router = router.merge(qr_pass_route(state.clone(), &rate_limiters));
    }

    // Swagger UI (only available when all modules are compiled)
    #[cfg(feature = "full")]
    {
        router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    let router = router
        // Prometheus metrics endpoint — protected by METRICS_TOKEN env var when set
        .route(
            "/metrics",
            get(move |req: Request<Body>| async move {
                if let Ok(expected) = std::env::var("METRICS_TOKEN")
                    && !expected.is_empty() {
                        let authorized = req
                            .headers()
                            .get(header::AUTHORIZATION)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.strip_prefix("Bearer "))
                            .is_some_and(|token| {
                                // Constant-time comparison to prevent timing attacks (issue #114)
                                use subtle::ConstantTimeEq;
                                token.as_bytes().ct_eq(expected.as_bytes()).into()
                            });
                        if !authorized {
                            return (
                                StatusCode::UNAUTHORIZED,
                                [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                                "Unauthorized".to_string(),
                            );
                        }
                    }
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                    metrics_handle_clone.render(),
                )
            }),
        )
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
        // Response compression (zstd + brotli + gzip) — negotiated via Accept-Encoding
        .layer(CompressionLayer::new().gzip(true).br(true).zstd(true))
        // Global rate limit — 100 req/s with burst 200
        .layer(axum::middleware::from_fn(move |req, next| {
            crate::rate_limit::rate_limit_middleware(global_limiter.clone(), req, next)
        }))
        // Security headers applied to every response
        .layer(axum::middleware::from_fn(security_headers_middleware));

    // API version header on all responses (X-API-Version, optional Sunset)
    #[cfg(feature = "mod-api-versioning")]
    let router = router.layer(axum::middleware::from_fn(
        versioning::api_version_middleware,
    ));

    let router = router
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
                    HeaderName::from_static("x-api-key"),
                    HeaderName::from_static("x-requested-with"),
                ])
                .expose_headers([HeaderName::from_static("x-request-id")])
                .allow_credentials(true),
        )
        // Assign a unique x-request-id to every inbound request (UUID v4)
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        // JWT revocation store — made available to every handler via
        // `request.extensions()`.  `AuthUser` (see `crate::jwt`) consults
        // this on every token validation.  The backend (in-memory vs
        // Redis) is decided once in `main::build_revocation_store`.
        .layer(Extension(revocation_store))
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

// Middleware re-exports from system module
use system::{http_metrics_middleware, request_id_tracing_middleware, security_headers_middleware};

// ═══════════════════════════════════════════════════════════════════════════════
// AUTH MIDDLEWARE
// ═══════════════════════════════════════════════════════════════════════════════

async fn auth_middleware(
    State(state): State<SharedState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, Json<ApiResponse<()>>)> {
    // Check for X-API-Key header first (alternative to Bearer token)
    if let Some(api_key) = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
    {
        let state_guard = state.read().await;
        if let Some((user_id, api_key_id)) =
            security::validate_api_key_detailed(&state_guard.db, api_key).await
        {
            // Verify user is still active
            match state_guard.db.get_user(&user_id.to_string()).await {
                Ok(Some(u)) if u.is_active => {
                    drop(state_guard);
                    request.extensions_mut().insert(AuthUser {
                        user_id,
                        api_key_id: Some(api_key_id),
                    });
                    return Ok(next.run(request).await);
                }
                _ => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ApiResponse::error("UNAUTHORIZED", "Invalid API key")),
                    ));
                }
            }
        }
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::error("UNAUTHORIZED", "Invalid API key")),
        ));
    }

    // Extract token: prefer Authorization header, fall back to httpOnly cookie.
    // This allows both API clients (Bearer header) and browser SPAs (cookie) to
    // authenticate. Header takes precedence when both are present.
    let bearer_token: Option<String> = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(String::from);

    let cookie_token: Option<String> = request
        .headers()
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{}=", auth::AUTH_COOKIE_NAME))
                    .map(std::string::ToString::to_string)
            })
        });

    let is_cookie_auth = bearer_token.is_none() && cookie_token.is_some();
    let token_owned = match bearer_token.or(cookie_token) {
        Some(t) if !t.is_empty() => t,
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
    let token = token_owned.as_str();

    // CSRF protection: when authenticating via cookie, require the
    // X-Requested-With header. This ensures the request was made via
    // JavaScript (which triggers CORS preflight) rather than a plain
    // form submission from a malicious site.
    if is_cookie_auth {
        let has_csrf_header = request
            .headers()
            .get("x-requested-with")
            .and_then(|v| v.to_str().ok())
            .is_some();
        if !has_csrf_header {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(
                    "CSRF_VALIDATION_FAILED",
                    "X-Requested-With header required for cookie authentication",
                )),
            ));
        }
    }

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
    match state_guard.db.get_user(&session.user_id.to_string()).await {
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
                Json(ApiResponse::error("UNAUTHORIZED", "User not found")),
            ));
        }
    }
    drop(state_guard);

    // Insert user info into request extensions
    request.extensions_mut().insert(AuthUser {
        user_id: session.user_id,
        api_key_id: None,
    });

    Ok(next.run(request).await)
}

// ═══════════════════════════════════════════════════════════════════════════════
// PER-IDENTITY RATE-LIMIT ROUTER (T-1743)
// ═══════════════════════════════════════════════════════════════════════════════

/// Pick the [`IdentityBucketKind`] for a protected-router request.
///
/// * `/api/v1/admin/*`                                  → `Admin`
/// * `GET` / `HEAD`                                      → `Read`
/// * anything else (POST / PUT / PATCH / DELETE / …)     → `Mutation`
fn classify_protected_bucket(method: &axum::http::Method, path: &str) -> IdentityBucketKind {
    if path.starts_with("/api/v1/admin/") || path == "/api/v1/admin" {
        IdentityBucketKind::Admin
    } else if method == axum::http::Method::GET || method == axum::http::Method::HEAD {
        IdentityBucketKind::Read
    } else {
        IdentityBucketKind::Mutation
    }
}

/// Per-identity rate-limit middleware for authenticated (protected) routes.
///
/// Delegates to [`identity_rate_limit_middleware`] after resolving the bucket
/// kind from the request method + path.  Runs INSIDE `auth_middleware` so the
/// [`AuthUser`] extension is present by the time the identity limiter fires.
async fn protected_identity_rate_limit_middleware(
    limiters: std::sync::Arc<IdentityRateLimiters>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let kind = classify_protected_bucket(request.method(), request.uri().path());
    identity_rate_limit_middleware(limiters, kind, request, next).await
}

// Health & system handler re-exports from system module
use system::{
    handshake, health_check, liveness_check, readiness_check, server_status, system_maintenance,
    system_version, v1_discover, v1_health, v1_health_detailed, v1_health_info, v1_health_live,
    v1_health_ready, v1_server_status,
};

// ═══════════════════════════════════════════════════════════════════════════════
// TOKEN GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Generate a cryptographically random access token (32 bytes, hex-encoded).
///
/// UUIDs v4 have a fixed structure that reduces effective entropy to ~122 bits.
/// Using a raw 256-bit random value is both simpler and more secure.
pub fn generate_access_token() -> String {
    let mut bytes = [0u8; 32];
    rand::Rng::fill_bytes(&mut rand::rng(), &mut bytes);
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
    argon2::Params::new(65_536, 3, 4, None).expect("OWASP Argon2 params are statically valid")
}

/// Hash a password using Argon2id, wrapped in `spawn_blocking` to avoid
/// blocking the async runtime (issue #117).
#[allow(clippy::result_large_err)]
pub async fn hash_password(
    password: &str,
) -> Result<String, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    let password = password.to_string();
    tokio::task::spawn_blocking(move || hash_password_sync(&password))
        .await
        .map_err(|e| {
            tracing::error!("spawn_blocking failed for password hashing: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            )
        })?
}

/// Hash a password using Argon2id, returning an `anyhow::Result`.
/// Wrapped in `spawn_blocking` (issue #117).
pub async fn hash_password_simple(password: &str) -> anyhow::Result<String> {
    let password = password.to_string();
    tokio::task::spawn_blocking(move || hash_password_simple_sync(&password))
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking failed: {e}"))?
}

/// Verify a password against a hash via `spawn_blocking` (issue #117).
pub async fn verify_password(password: &str, hash: &str) -> bool {
    let password = password.to_string();
    let hash = hash.to_string();
    tokio::task::spawn_blocking(move || verify_password_sync(&password, &hash))
        .await
        .unwrap_or(false)
}

// ── Synchronous inner functions (run on blocking threadpool) ─────────────

#[allow(clippy::result_large_err)]
fn hash_password_sync(
    password: &str,
) -> Result<String, (StatusCode, Json<ApiResponse<LoginResponse>>)> {
    use argon2::{
        Algorithm, Argon2, Version,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
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

fn hash_password_simple_sync(password: &str) -> anyhow::Result<String> {
    use argon2::{
        Algorithm, Argon2, Version,
        password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params())
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("Argon2 hashing failed: {e}"))
}

fn verify_password_sync(password: &str, hash: &str) -> bool {
    use argon2::{
        Algorithm, Argon2, Version,
        password_hash::{PasswordHash, PasswordVerifier},
    };
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params())
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// ═══════════════════════════════════════════════════════════════════════════════
// BOOKING UPDATE (PATCH /api/v1/bookings/{id})
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tenant_scope_tests {
    //! T-1731 pre-flip hardening: unit tests for the tenant-resolution and
    //! read-path guards.  `resolve_tenant_id` is exercised end-to-end against
    //! an in-memory database so we cover both "user has tenant" and "user has
    //! no tenant" branches.  `matches_tenant` is tested in-process because it
    //! is a pure function.

    use super::*;
    use crate::config::ServerConfig;
    use crate::db::{Database, DatabaseConfig};
    use parkhub_common::models::{User, UserPreferences, UserRole};

    struct StateHarness {
        state: AppState,
        _dir: tempfile::TempDir,
    }

    fn build_state() -> StateHarness {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&db_config).expect("open test db");
        let state = AppState {
            config: ServerConfig::default(),
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
            fleet_events: crate::api::sse::FleetEventBroadcaster::new(),
            revocation_store: crate::jwt::TokenRevocationList::new(),
        };
        StateHarness { state, _dir: dir }
    }

    fn make_user(tenant_id: Option<String>) -> User {
        User {
            id: Uuid::new_v4(),
            username: "t1731".to_string(),
            email: "t1731@example.test".to_string(),
            name: "Tenant Test".to_string(),
            password_hash: "x".to_string(),
            role: UserRole::User,
            is_active: true,
            phone: None,
            picture: None,
            preferences: UserPreferences::default(),
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_login: None,
            tenant_id,
            accessibility_needs: None,
            cost_center: None,
            department: None,
            settings: None,
        }
    }

    #[tokio::test]
    async fn resolve_tenant_id_returns_user_tenant_when_set() {
        let h = build_state();
        let user = make_user(Some("tenant-acme".to_string()));
        let uid = user.id;
        h.state.db.save_user(&user).await.expect("save user");

        let resolved = resolve_tenant_id(&h.state, uid).await;
        assert_eq!(
            resolved.as_deref(),
            Some("tenant-acme"),
            "caller's tenant_id must propagate via resolver"
        );
    }

    #[tokio::test]
    async fn resolve_tenant_id_returns_none_when_user_is_platform_scope() {
        let h = build_state();
        let user = make_user(None);
        let uid = user.id;
        h.state.db.save_user(&user).await.expect("save user");

        let resolved = resolve_tenant_id(&h.state, uid).await;
        assert!(
            resolved.is_none(),
            "platform-admin / unbound users must resolve to None (flag-off default)"
        );
    }

    #[tokio::test]
    async fn resolve_tenant_id_returns_none_for_missing_user() {
        let h = build_state();
        let ghost = Uuid::new_v4();
        let resolved = resolve_tenant_id(&h.state, ghost).await;
        assert!(
            resolved.is_none(),
            "missing user must not panic and must default to None"
        );
    }

    #[test]
    fn matches_tenant_platform_admin_sees_all() {
        // caller == None (platform scope) sees every entity regardless of tenant
        assert!(matches_tenant(None, None));
        assert!(matches_tenant(Some("t-a"), None));
        assert!(matches_tenant(Some("t-b"), None));
    }

    #[test]
    fn matches_tenant_tenant_admin_sees_only_own() {
        assert!(matches_tenant(Some("t-a"), Some("t-a")));
        assert!(!matches_tenant(Some("t-b"), Some("t-a")));
        // entity with no tenant is NOT visible to a tenant-bound caller
        assert!(!matches_tenant(None, Some("t-a")));
    }
}
