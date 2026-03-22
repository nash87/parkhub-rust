//! HTTP API Routes
//!
//! `RESTful` API for the parking system.

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Extension, Json, Router,
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
#[cfg(feature = "mod-email")]
use crate::email;
use crate::metrics;
#[cfg(feature = "full")]
use crate::openapi::ApiDoc;
use crate::rate_limit::{ip_rate_limit_middleware, EndpointRateLimiters};
use crate::static_files;

/// Maximum allowed request body size: 4 MiB.
/// Raised from 1 MiB to accommodate base64-encoded vehicle photos (max 2 MB raw
/// ≈ 2.7 MB base64 + JSON envelope).  Normal API payloads remain well under this.
const MAX_REQUEST_BODY_BYTES: usize = 4 * 1024 * 1024; // 4 MiB

/// Maximum raw photo size in bytes (2 MB).
#[cfg(feature = "mod-vehicles")]
pub const MAX_PHOTO_BYTES: usize = 2 * 1024 * 1024;

/// German standard VAT rate (19% — Umsatzsteuergesetz § 12 Abs. 1)
#[cfg(feature = "mod-bookings")]
pub(super) const VAT_RATE: f64 = 0.19;

use parkhub_common::{ApiResponse, LoginResponse, UserRole};

use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Submodules
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "mod-absences")]
pub mod absences;
#[cfg(feature = "mod-api-docs")]
pub mod api_docs;
#[cfg(feature = "mod-accessible")]
pub mod accessible;
pub mod admin;
pub mod admin_ext;
pub mod admin_handlers;
#[cfg(feature = "mod-analytics")]
pub mod analytics;
#[cfg(feature = "mod-announcements")]
pub mod announcements;
pub mod auth;
#[cfg(feature = "mod-cost-center")]
pub mod billing;
#[cfg(feature = "mod-bookings")]
pub mod bookings;
#[cfg(feature = "mod-branding")]
pub mod branding;
#[cfg(feature = "mod-calendar")]
pub mod calendar;
#[cfg(feature = "mod-credits")]
pub mod credits;
#[cfg(feature = "mod-data-import")]
pub mod data_management;
#[cfg(feature = "mod-dynamic-pricing")]
pub mod dynamic_pricing;
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
#[cfg(feature = "mod-notifications")]
pub mod notification_channels;
#[cfg(feature = "mod-notifications")]
pub mod notifications;
#[cfg(feature = "mod-oauth")]
pub mod oauth;
#[cfg(feature = "mod-parking-pass")]
pub mod parking_pass;
#[cfg(feature = "mod-operating-hours")]
pub mod operating_hours;
#[cfg(feature = "mod-payments")]
pub mod payments;
#[cfg(feature = "mod-push")]
pub mod push;
#[cfg(feature = "mod-pwa")]
pub mod pwa;
#[cfg(feature = "mod-qr")]
pub mod qr;
pub mod rate_dashboard;
#[cfg(feature = "mod-recommendations")]
pub mod recommendations;
#[cfg(feature = "mod-recurring")]
pub mod recurring;
pub mod security;
#[cfg(feature = "mod-settings")]
pub mod settings;
pub mod setup;
#[cfg(feature = "mod-social")]
mod social;
#[cfg(feature = "mod-stripe")]
pub mod stripe;
#[cfg(feature = "mod-swap")]
pub mod swap;
pub mod system;
#[cfg(feature = "mod-team")]
pub mod team;
#[cfg(feature = "mod-multi-tenant")]
pub mod tenants;
#[cfg(feature = "mod-translations")]
pub mod translations;
mod users;
#[cfg(feature = "mod-vehicles")]
pub mod vehicles;
#[cfg(feature = "mod-visitors")]
pub mod visitors;
#[cfg(feature = "mod-waitlist")]
pub mod waitlist;
#[cfg(feature = "mod-waitlist-ext")]
pub mod waitlist_ext;
#[cfg(feature = "mod-webhooks")]
pub mod webhooks;
pub mod ws;
#[cfg(feature = "mod-zones")]
pub mod zones;

// Re-import handler functions so the router can reference them unqualified.
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
use auth::{forgot_password, login, logout, refresh_token, register, reset_password};
#[cfg(feature = "mod-bookings")]
pub use bookings::{
    booking_checkin, cancel_booking, create_booking, get_booking, get_booking_invoice,
    list_bookings, quick_book, update_booking,
};
#[cfg(feature = "mod-calendar")]
use calendar::{calendar_events, user_calendar_ics};
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
use guest::{admin_cancel_guest_booking, admin_list_guest_bookings, create_guest_booking};
#[cfg(feature = "mod-history")]
use history::{booking_history, booking_stats};
#[cfg(feature = "mod-import")]
use import::import_users_csv;
use lots::{
    create_lot, create_slot, delete_lot, delete_slot, get_lot, get_lot_pricing, get_lot_slots,
    list_lots, update_lot, update_lot_pricing, update_slot,
};
#[cfg(feature = "mod-notifications")]
use notifications::{list_notifications, mark_all_notifications_read, mark_notification_read};
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
#[cfg(feature = "mod-parking-pass")]
use parking_pass::{get_booking_pass, list_my_passes, verify_pass};
#[cfg(feature = "mod-webhooks")]
use webhooks::{create_webhook, delete_webhook, list_webhooks, test_webhook, update_webhook};
#[cfg(feature = "mod-zones")]
use zones::{create_zone, delete_zone, list_zones, update_zone};

// Re-exports from extracted modules (Phase 3)
use admin_handlers::{
    admin_audit_log, admin_audit_log_export, admin_delete_user, admin_get_auto_release,
    admin_get_email_settings, admin_get_privacy, admin_heatmap, admin_list_bookings,
    admin_list_users, admin_reports, admin_reset, admin_stats, admin_update_auto_release,
    admin_update_email_settings, admin_update_privacy, admin_update_user, admin_update_user_role,
    admin_update_user_status,
};
use lots_ext::{admin_dashboard_charts, lot_qr_code};
use misc::{
    get_impressum, get_impressum_admin, public_display, public_occupancy, update_impressum,
};
use users::{
    change_password, gdpr_delete_account, gdpr_export_data, get_current_user, get_user,
    get_user_preferences, update_current_user, update_user_preferences, user_stats,
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

/// `GET /api/v1/modules` — compile-time module feature introspection.
///
/// Returns which optional modules are compiled into this binary.
/// Always available (no auth required, no feature gate).
async fn list_module_features() -> impl IntoResponse {
    let mut modules = serde_json::Map::new();

    modules.insert("bookings".into(), cfg!(feature = "mod-bookings").into());
    modules.insert("vehicles".into(), cfg!(feature = "mod-vehicles").into());
    modules.insert("absences".into(), cfg!(feature = "mod-absences").into());
    modules.insert("branding".into(), cfg!(feature = "mod-branding").into());
    modules.insert("import".into(), cfg!(feature = "mod-import").into());
    modules.insert(
        "data-import".into(),
        cfg!(feature = "mod-data-import").into(),
    );
    modules.insert("fleet".into(), cfg!(feature = "mod-fleet").into());
    modules.insert("qr".into(), cfg!(feature = "mod-qr").into());
    modules.insert("pwa".into(), cfg!(feature = "mod-pwa").into());
    modules.insert("payments".into(), cfg!(feature = "mod-payments").into());
    modules.insert("webhooks".into(), cfg!(feature = "mod-webhooks").into());
    modules.insert(
        "notifications".into(),
        cfg!(feature = "mod-notifications").into(),
    );
    modules.insert(
        "announcements".into(),
        cfg!(feature = "mod-announcements").into(),
    );
    modules.insert("recurring".into(), cfg!(feature = "mod-recurring").into());
    modules.insert("guest".into(), cfg!(feature = "mod-guest").into());
    modules.insert("calendar".into(), cfg!(feature = "mod-calendar").into());
    modules.insert("team".into(), cfg!(feature = "mod-team").into());
    modules.insert("settings".into(), cfg!(feature = "mod-settings").into());
    modules.insert("jobs".into(), cfg!(feature = "mod-jobs").into());
    modules.insert("swap".into(), cfg!(feature = "mod-swap").into());
    modules.insert("waitlist".into(), cfg!(feature = "mod-waitlist").into());
    modules.insert("zones".into(), cfg!(feature = "mod-zones").into());
    modules.insert("credits".into(), cfg!(feature = "mod-credits").into());
    modules.insert("email".into(), cfg!(feature = "mod-email").into());
    modules.insert("export".into(), cfg!(feature = "mod-export").into());
    modules.insert("favorites".into(), cfg!(feature = "mod-favorites").into());
    modules.insert("push".into(), cfg!(feature = "mod-push").into());
    modules.insert(
        "recommendations".into(),
        cfg!(feature = "mod-recommendations").into(),
    );
    modules.insert(
        "translations".into(),
        cfg!(feature = "mod-translations").into(),
    );
    modules.insert("social".into(), cfg!(feature = "mod-social").into());
    modules.insert("themes".into(), cfg!(feature = "mod-themes").into());
    modules.insert("oauth".into(), cfg!(feature = "mod-oauth").into());
    modules.insert("invoices".into(), cfg!(feature = "mod-invoices").into());
    modules.insert(
        "dynamic-pricing".into(),
        cfg!(feature = "mod-dynamic-pricing").into(),
    );
    modules.insert(
        "operating-hours".into(),
        cfg!(feature = "mod-operating-hours").into(),
    );
    modules.insert("websocket".into(), cfg!(feature = "mod-websocket").into());
    modules.insert(
        "lobby-display".into(),
        cfg!(feature = "mod-lobby-display").into(),
    );
    modules.insert(
        "setup-wizard".into(),
        cfg!(feature = "mod-setup-wizard").into(),
    );
    modules.insert("map".into(), cfg!(feature = "mod-map").into());
    modules.insert("stripe".into(), cfg!(feature = "mod-stripe").into());
    modules.insert(
        "multi-tenant".into(),
        cfg!(feature = "mod-multi-tenant").into(),
    );
    modules.insert("accessible".into(), cfg!(feature = "mod-accessible").into());
    modules.insert(
        "maintenance".into(),
        cfg!(feature = "mod-maintenance").into(),
    );
    modules.insert(
        "cost-center".into(),
        cfg!(feature = "mod-cost-center").into(),
    );
    modules.insert("visitors".into(), cfg!(feature = "mod-visitors").into());
    modules.insert(
        "ev-charging".into(),
        cfg!(feature = "mod-ev-charging").into(),
    );
    modules.insert("history".into(), cfg!(feature = "mod-history").into());
    modules.insert("geofence".into(), cfg!(feature = "mod-geofence").into());
    modules.insert(
        "waitlist-ext".into(),
        cfg!(feature = "mod-waitlist-ext").into(),
    );
    modules.insert(
        "parking-pass".into(),
        cfg!(feature = "mod-parking-pass").into(),
    );
    modules.insert(
        "api-docs".into(),
        cfg!(feature = "mod-api-docs").into(),
    );

    Json(serde_json::json!({
        "modules": modules,
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Create the API router with `OpenAPI` docs and metrics.
/// Returns (router, `demo_state`) so the demo state can be used for scheduled resets.
#[allow(unused_mut, unused_variables)]
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

    // POST /api/v1/auth/logout — clears httpOnly cookie, invalidates session
    let logout_route = Router::new().route("/api/v1/auth/logout", post(logout));

    // GET /api/v1/bookings/:id/qr — 10 requests per minute per IP (QR pass generation)
    #[cfg(feature = "mod-qr")]
    let qr_route = {
        let qr_limiter = rate_limiters.qr_pass.clone();
        Router::new()
            .route("/api/v1/bookings/{id}/qr", get(qr::booking_qr_code))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .route_layer(middleware::from_fn(move |req, next| {
                ip_rate_limit_middleware(qr_limiter.clone(), req, next)
            }))
    };

    // Remaining public routes (no rate limiting needed)
    let mut public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))
        .route("/health/detailed", get(admin_ext::detailed_health_check))
        .route("/handshake", post(handshake))
        .route("/status", get(server_status))
        // Legal — public (DDG § 5 requires Impressum to be freely accessible)
        .route("/api/v1/legal/impressum", get(get_impressum))
        // Module feature flags — public (compile-time feature introspection)
        .route("/api/v1/modules", get(list_module_features))
        // Setup wizard — only works before initial setup is completed
        .route("/api/v1/setup/status", get(setup::setup_status))
        .route("/api/v1/setup", post(setup::setup_init))
        // Public occupancy display (no auth)
        .route("/api/v1/public/occupancy", get(public_occupancy))
        .route("/api/v1/public/display", get(public_display))
        // System info (public — no auth needed for version/maintenance checks)
        .route("/api/v1/system/version", get(system_version))
        .route("/api/v1/system/maintenance", get(system_maintenance))
        // WebSocket real-time events
        .route("/api/v1/ws", get(ws::ws_handler));

    // Setup wizard (multi-step onboarding) — public for initial setup
    #[cfg(feature = "mod-setup-wizard")]
    {
        public_routes = public_routes
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
        public_routes = public_routes.merge(lobby_route);
    }

    // Interactive API documentation (public)
    #[cfg(feature = "mod-api-docs")]
    {
        public_routes = public_routes
            .route("/api/v1/docs", get(api_docs::api_docs_ui))
            .route(
                "/api/v1/docs/openapi.json",
                get(api_docs::api_docs_openapi_json),
            );
    }

    // Public pass verification (no auth needed — used by QR scan)
    #[cfg(feature = "mod-parking-pass")]
    {
        public_routes =
            public_routes.route("/api/v1/pass/verify/{code}", get(verify_pass));
    }

    // Feature-gated public routes
    #[cfg(feature = "mod-settings")]
    {
        // Feature flags — public (frontend needs to know which features are enabled)
        public_routes = public_routes
            .route("/api/v1/features", get(get_features))
            // Theme — public (frontend needs theme before auth for login page styling)
            .route("/api/v1/theme", get(get_public_theme));
    }
    #[cfg(feature = "mod-announcements")]
    {
        // Announcements — public (active announcements visible without auth)
        public_routes = public_routes.route(
            "/api/v1/announcements/active",
            get(get_active_announcements),
        );
    }
    #[cfg(feature = "mod-push")]
    {
        // VAPID public key (no auth — frontend needs it before login)
        public_routes = public_routes.route("/api/v1/push/vapid-key", get(push::get_vapid_key));
    }
    #[cfg(feature = "mod-map")]
    {
        // Map markers — public (frontend shows map without auth)
        public_routes = public_routes.route("/api/v1/lots/map", get(map::list_lot_markers));
    }
    #[cfg(feature = "mod-stripe")]
    {
        // Stripe webhook (no auth — Stripe calls this endpoint directly)
        // Stripe config (public — frontend needs to know if Stripe is available)
        public_routes = public_routes
            .route(
                "/api/v1/payments/webhook",
                post(stripe::stripe_webhook).layer(Extension(stripe::new_checkout_store())),
            )
            .route("/api/v1/payments/config", get(stripe::stripe_config));
    }
    #[cfg(feature = "mod-pwa")]
    {
        // PWA manifest and service worker (no auth)
        public_routes = public_routes
            .route("/manifest.json", get(pwa::pwa_manifest))
            .route("/sw.js", get(pwa::service_worker));
    }
    #[cfg(feature = "mod-branding")]
    {
        // Branding logo (public, cached)
        public_routes =
            public_routes.route("/api/v1/branding/logo", get(branding::get_branding_logo));
    }
    #[cfg(feature = "mod-oauth")]
    {
        // OAuth: providers list (public, no auth) + redirect + callback
        public_routes = public_routes
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

    // Protected routes (auth required) — core user + admin routes
    let mut protected_routes = Router::new()
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
        );

    // Dynamic pricing (occupancy-based) — user-facing read endpoint
    #[cfg(feature = "mod-dynamic-pricing")]
    {
        protected_routes = protected_routes.route(
            "/api/v1/lots/{id}/pricing/dynamic",
            get(dynamic_pricing::get_dynamic_pricing),
        );
    }

    // Operating hours — user-facing read endpoint
    #[cfg(feature = "mod-operating-hours")]
    {
        protected_routes = protected_routes.route(
            "/api/v1/lots/{id}/hours",
            get(operating_hours::get_operating_hours),
        );
    }

    protected_routes = protected_routes
        // QR code for lot
        .route("/api/v1/lots/{id}/qr", get(lot_qr_code))
        // User stats & preferences
        .route("/api/v1/user/stats", get(user_stats))
        .route(
            "/api/v1/user/preferences",
            get(get_user_preferences).put(update_user_preferences),
        )
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
        );

    // ── Admin routes (guarded by admin_middleware) ────────────────────────
    // All /api/v1/admin/* routes are grouped under a shared admin_middleware
    // layer (issue #109) providing defense-in-depth: even if a handler forgets
    // to call check_admin(), the middleware rejects non-admin users.
    let admin_routes = Router::new()
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

    #[cfg(feature = "mod-analytics")]
    let admin_routes = admin_routes.route(
        "/api/v1/admin/analytics/overview",
        get(analytics::analytics_overview),
    );

    let admin_routes = admin_routes
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
    let admin_routes = admin_routes
        .route(
            "/api/v1/admin/tenants",
            get(tenants::list_tenants).post(tenants::create_tenant),
        )
        .route("/api/v1/admin/tenants/{id}", put(tenants::update_tenant));

    #[cfg(feature = "mod-geofence")]
    let admin_routes =
        admin_routes.route("/api/v1/admin/lots/{id}/geofence", put(admin_set_geofence));

    let admin_routes = admin_routes.route_layer(middleware::from_fn_with_state(
        state.clone(),
        admin_middleware,
    ));

    // Merge admin routes into protected routes
    protected_routes = protected_routes.merge(admin_routes);

    // ── Feature-gated protected routes ──────────────────────────────────────

    #[cfg(feature = "mod-bookings")]
    {
        protected_routes = protected_routes
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

    #[cfg(feature = "mod-history")]
    {
        protected_routes = protected_routes
            .route("/api/v1/bookings/history", get(booking_history))
            .route("/api/v1/bookings/stats", get(booking_stats));
    }

    #[cfg(feature = "mod-geofence")]
    {
        protected_routes = protected_routes
            .route("/api/v1/geofence/check-in", post(geofence_check_in))
            .route("/api/v1/lots/{id}/geofence", get(get_lot_geofence));
    }

    #[cfg(feature = "mod-waitlist-ext")]
    {
        protected_routes = protected_routes
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
        protected_routes = protected_routes
            .route("/api/v1/bookings/{id}/pass", get(get_booking_pass))
            .route("/api/v1/me/passes", get(list_my_passes));
    }

    #[cfg(feature = "mod-invoices")]
    {
        // PDF invoice download
        protected_routes = protected_routes.route(
            "/api/v1/bookings/{id}/invoice/pdf",
            get(invoices::get_booking_invoice_pdf),
        );
    }

    #[cfg(feature = "mod-qr")]
    {
        // QR code for individual slot
        protected_routes = protected_routes.route(
            "/api/v1/lots/{lot_id}/slots/{slot_id}/qr",
            get(qr::slot_qr_code),
        );
    }

    #[cfg(feature = "mod-zones")]
    {
        // Zones (admin-only CRUD, nested under lots)
        protected_routes = protected_routes
            .route(
                "/api/v1/lots/{lot_id}/zones",
                get(list_zones).post(create_zone),
            )
            .route(
                "/api/v1/lots/{lot_id}/zones/{zone_id}",
                put(update_zone).delete(delete_zone),
            );
    }

    #[cfg(feature = "mod-recommendations")]
    {
        // Smart parking recommendations
        protected_routes = protected_routes
            .route("/api/v1/bookings/recommendations", get(get_recommendations))
            .route(
                "/api/v1/recommendations/stats",
                get(get_recommendation_stats),
            );
    }

    #[cfg(feature = "mod-vehicles")]
    {
        protected_routes = protected_routes
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

    #[cfg(feature = "mod-credits")]
    {
        // Credits
        protected_routes = protected_routes
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
        // Favorites (user-authenticated)
        protected_routes = protected_routes
            .route(
                "/api/v1/user/favorites",
                get(list_favorites).post(add_favorite),
            )
            .route("/api/v1/user/favorites/{slot_id}", delete(remove_favorite));
    }

    #[cfg(feature = "mod-settings")]
    {
        // Admin-only: feature flags management
        protected_routes = protected_routes
            .route(
                "/api/v1/admin/features",
                get(admin_get_features).put(admin_update_features),
            )
            // Admin-only: system settings
            .route(
                "/api/v1/admin/settings",
                get(admin_get_settings).put(admin_update_settings),
            )
            .route("/api/v1/admin/settings/use-case", get(admin_get_use_case));
    }

    #[cfg(feature = "mod-dynamic-pricing")]
    {
        // Admin-only: dynamic pricing rules management
        protected_routes = protected_routes.route(
            "/api/v1/admin/lots/{id}/pricing/dynamic",
            get(dynamic_pricing::admin_get_dynamic_pricing_rules)
                .put(dynamic_pricing::admin_update_dynamic_pricing_rules),
        );
    }

    #[cfg(feature = "mod-operating-hours")]
    {
        // Admin-only: operating hours management
        protected_routes = protected_routes.route(
            "/api/v1/admin/lots/{id}/hours",
            put(operating_hours::admin_update_operating_hours),
        );
    }

    #[cfg(feature = "mod-import")]
    {
        // Admin-only: CSV import
        protected_routes =
            protected_routes.route("/api/v1/admin/users/import", post(import_users_csv));
    }

    #[cfg(feature = "mod-export")]
    {
        protected_routes = protected_routes
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
        protected_routes = protected_routes
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
        protected_routes = protected_routes
            .route("/api/v1/admin/fleet", get(fleet::admin_fleet_list))
            .route("/api/v1/admin/fleet/stats", get(fleet::admin_fleet_stats))
            .route(
                "/api/v1/admin/fleet/{id}/flag",
                put(fleet::admin_fleet_flag),
            );
    }

    #[cfg(feature = "mod-accessible")]
    {
        protected_routes = protected_routes
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
        protected_routes = protected_routes
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
        protected_routes = protected_routes
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
        // EV Charging
        protected_routes = protected_routes
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
        // Absences (user-scoped)
        protected_routes = protected_routes
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

    // Absence iCal import needs both absences + import modules
    #[cfg(all(feature = "mod-absences", feature = "mod-import"))]
    {
        protected_routes = protected_routes.route(
            "/api/v1/absences/import/ical",
            post(import::import_absences_ical),
        );
    }

    #[cfg(feature = "mod-team")]
    {
        // Team view
        protected_routes = protected_routes
            .route("/api/v1/team/today", get(team_today))
            .route("/api/v1/team", get(team_list));
    }

    #[cfg(feature = "mod-announcements")]
    {
        // Admin-only: announcements management
        protected_routes = protected_routes
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
        // Notifications (user-scoped)
        protected_routes = protected_routes
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

    #[cfg(feature = "mod-waitlist")]
    {
        // Waitlist
        protected_routes = protected_routes
            .route("/api/v1/waitlist", get(list_waitlist).post(join_waitlist))
            .route("/api/v1/waitlist/{id}", delete(leave_waitlist));
    }

    #[cfg(feature = "mod-swap")]
    {
        // Swap requests
        protected_routes = protected_routes
            .route("/api/v1/swap-requests", get(list_swap_requests))
            .route(
                "/api/v1/bookings/{id}/swap-request",
                post(create_swap_request),
            )
            .route("/api/v1/swap-requests/{id}", put(update_swap_request));
    }

    #[cfg(feature = "mod-recurring")]
    {
        // Recurring bookings
        protected_routes = protected_routes
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
        // Guest bookings
        protected_routes = protected_routes
            .route("/api/v1/bookings/guest", post(create_guest_booking))
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
        // Visitor pre-registration
        protected_routes = protected_routes
            .route("/api/v1/visitors/register", post(register_visitor))
            .route("/api/v1/visitors", get(list_my_visitors))
            .route("/api/v1/visitors/{id}/check-in", put(check_in_visitor))
            .route("/api/v1/visitors/{id}", delete(cancel_visitor))
            .route("/api/v1/admin/visitors", get(admin_list_visitors));
    }

    #[cfg(feature = "mod-calendar")]
    {
        // Calendar
        protected_routes = protected_routes
            .route("/api/v1/calendar/events", get(calendar_events))
            // iCal export for user's bookings
            .route("/api/v1/user/calendar.ics", get(user_calendar_ics));
    }

    #[cfg(feature = "mod-webhooks")]
    {
        // Admin-only: webhooks
        protected_routes = protected_routes
            .route("/api/v1/webhooks", get(list_webhooks).post(create_webhook))
            .route(
                "/api/v1/webhooks/{id}",
                put(update_webhook).delete(delete_webhook),
            )
            .route("/api/v1/webhooks/{id}/test", post(test_webhook));
    }

    #[cfg(feature = "mod-push")]
    {
        // Push notifications
        protected_routes = protected_routes
            .route("/api/v1/push/subscribe", post(push::subscribe))
            .route("/api/v1/push/unsubscribe", delete(push::unsubscribe));
    }

    #[cfg(feature = "mod-branding")]
    {
        // Admin: branding config
        protected_routes = protected_routes
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
        // Admin: set lot coordinates for map view
        protected_routes = protected_routes.route(
            "/api/v1/admin/lots/{id}/location",
            put(map::set_lot_location),
        );
    }

    #[cfg(feature = "mod-translations")]
    {
        // Translation management
        protected_routes = protected_routes
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

    #[cfg(feature = "mod-payments")]
    {
        // Payments (Stripe stub)
        protected_routes = protected_routes
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
        // Stripe checkout (authenticated routes)
        let stripe_store = stripe::new_checkout_store();
        protected_routes = protected_routes
            .route(
                "/api/v1/payments/create-checkout",
                post(stripe::create_checkout),
            )
            .route("/api/v1/payments/history", get(stripe::payment_history))
            .layer(Extension(stripe_store));
    }

    // Apply auth middleware to all protected routes
    let protected_routes = protected_routes.route_layer(middleware::from_fn_with_state(
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

    let mut router = Router::new()
        .merge(public_routes)
        .merge(login_route)
        .merge(register_route)
        .merge(forgot_route)
        .merge(refresh_route)
        .merge(reset_password_route)
        .merge(logout_route)
        .merge(demo_routes)
        .merge(protected_routes);

    #[cfg(feature = "mod-qr")]
    {
        router = router.merge(qr_route);
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
                if let Ok(expected) = std::env::var("METRICS_TOKEN") {
                    if !expected.is_empty() {
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
                    HeaderName::from_static("x-api-key"),
                    HeaderName::from_static("x-requested-with"),
                ])
                .expose_headers([HeaderName::from_static("x-request-id")])
                .allow_credentials(true),
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
        if let Some(user_id) = security::validate_api_key(&state_guard.db, api_key).await {
            // Verify user is still active
            match state_guard.db.get_user(&user_id.to_string()).await {
                Ok(Some(u)) if u.is_active => {
                    drop(state_guard);
                    request.extensions_mut().insert(AuthUser { user_id });
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
    });

    Ok(next.run(request).await)
}

// Health & system handler re-exports from system module
use system::{
    handshake, health_check, liveness_check, readiness_check, server_status, system_maintenance,
    system_version,
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

fn hash_password_simple_sync(password: &str) -> anyhow::Result<String> {
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

fn verify_password_sync(password: &str, hash: &str) -> bool {
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
// BOOKING UPDATE (PATCH /api/v1/bookings/{id})
// ═══════════════════════════════════════════════════════════════════════════════
