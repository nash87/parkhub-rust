//! Module metadata registry — makes the 90+ feature flags introspectable
//! as structured data the admin UI can render.
//!
//! The registry is a single source of truth for category, description,
//! config keys, UI deep-link, and inter-module dependencies. Keeping it
//! in one file (instead of scattering `register_metadata!` macros across
//! each module) is a deliberate choice: new modules are added by
//! editing one table, which keeps the ADMIN-facing surface coherent
//! and spares us a cross-cutting "forgot to register" class of bugs.
//!
//! Design principle: **nothing here calls out to the internet**. The
//! description strings ship in the binary; categories are compile-time
//! constants. Everything an admin sees about modules is locally sourced.

use axum::{
    extract::{Path, State},
    Json,
};
use parkhub_common::ApiResponse;
use serde::Serialize;
use utoipa::ToSchema;

use super::SharedState;

/// Semantic grouping used by the admin Modules Dashboard.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ModuleCategory {
    /// Fundamental resources — bookings, vehicles, zones.
    Core,
    /// Booking-side features — absences, recurring, guest, waitlist…
    Booking,
    /// Vehicle/fleet features.
    Vehicle,
    /// Admin-only surfaces — RBAC, audit export, analytics.
    Admin,
    /// Money flow — stripe, credits, invoices, dynamic pricing.
    Payment,
    /// External system integrations — webhooks, calendar, OAuth.
    Integration,
    /// Analytics + reporting.
    Analytics,
    /// Compliance, accessibility, GDPR workflows.
    Compliance,
    /// Notification channels — push, email, in-app.
    Notification,
    /// Enterprise multi-tenant, theming, plugins, widgets.
    Enterprise,
    /// Experimental / hardware-integration / map & geofence.
    Experimental,
}

/// Descriptor a module contributes to the registry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ModuleInfo {
    /// Stable slug (matches the feature flag without the `mod-` prefix).
    pub name: &'static str,
    pub category: ModuleCategory,
    pub description: &'static str,
    /// Compile-time `cfg!(feature = "mod-...")` result.
    pub enabled: bool,
    /// When true, the module's observable behavior can be toggled at
    /// runtime via an admin setting (without a rebuild). The UI surfaces
    /// a toggle only for these.
    pub runtime_toggleable: bool,
    /// Admin-settings keys the module consumes. Renderer links them.
    pub config_keys: &'static [&'static str],
    /// Deep-link into the user-facing UI (e.g. "/admin/plugins"). None
    /// means the module has no dedicated page and surfaces only via
    /// background behavior.
    pub ui_route: Option<&'static str>,
    /// Other module slugs that must also be enabled for this to work.
    pub depends_on: &'static [&'static str],
    /// Semver — tied to the workspace version.
    pub version: &'static str,
}

macro_rules! module {
    (
        name: $name:literal,
        cat: $cat:ident,
        desc: $desc:literal,
        enabled: $enabled:expr,
        $(runtime: $runtime:expr,)?
        $(config: [$($config:literal),* $(,)?],)?
        $(ui: $ui:literal,)?
        $(deps: [$($dep:literal),* $(,)?],)?
    ) => {
        ModuleInfo {
            name: $name,
            category: ModuleCategory::$cat,
            description: $desc,
            enabled: $enabled,
            runtime_toggleable: module!(@runtime $($runtime)?),
            config_keys: &[$($($config),*)?],
            ui_route: module!(@ui $($ui)?),
            depends_on: &[$($($dep),*)?],
            version: env!("CARGO_PKG_VERSION"),
        }
    };
    (@runtime $r:expr) => { $r };
    (@runtime) => { false };
    (@ui $u:literal) => { Some($u) };
    (@ui) => { None };
}

/// The single source of truth: an ordered list of every module's
/// metadata. The flags are resolved at compile time; the rest is
/// static strings, so this function is effectively free to call.
pub fn all_modules() -> Vec<ModuleInfo> {
    vec![
        // ── Core ───────────────────────────────────────────────────
        module!(name: "bookings", cat: Core, desc: "Create, edit, cancel, and query reservations.", enabled: cfg!(feature = "mod-bookings"), config: ["require_vehicle", "min_booking_duration_hours", "license_plate_mode"], ui: "/bookings",),
        module!(name: "vehicles", cat: Core, desc: "User-owned vehicle records with plates and default selection.", enabled: cfg!(feature = "mod-vehicles"), ui: "/vehicles",),
        module!(name: "zones", cat: Core, desc: "Organizational groupings of slots (Level A/B, EV row, visitor row).", enabled: cfg!(feature = "mod-zones"), ui: "/admin/zones",),
        // ── Booking-side ──────────────────────────────────────────
        module!(name: "absences", cat: Booking, desc: "Home-office, vacation, sick, training — drives auto-release of booked slots.", enabled: cfg!(feature = "mod-absences"), config: ["auto_release_on_absence"], ui: "/absences",),
        module!(name: "absence-approval", cat: Booking, desc: "Manager approval workflow for absence requests.", enabled: cfg!(feature = "mod-absence-approval"), deps: ["absences"],),
        module!(name: "recurring", cat: Booking, desc: "Weekly / monthly recurring bookings with rule engine.", enabled: cfg!(feature = "mod-recurring"), deps: ["bookings"],),
        module!(name: "guest", cat: Booking, desc: "Guest passes — share a time-limited booking with an external visitor.", enabled: cfg!(feature = "mod-guest"), deps: ["bookings"],),
        module!(name: "swap", cat: Booking, desc: "Peer-to-peer booking swaps between users.", enabled: cfg!(feature = "mod-swap"), deps: ["bookings"],),
        module!(name: "waitlist", cat: Booking, desc: "Waitlist notify-on-availability.", enabled: cfg!(feature = "mod-waitlist"),),
        module!(name: "waitlist-ext", cat: Booking, desc: "Advanced waitlist — priority, expiry, multi-slot.", enabled: cfg!(feature = "mod-waitlist-ext"), deps: ["waitlist"],),
        module!(name: "sharing", cat: Booking, desc: "Shareable booking links with QR + guest registration.", enabled: cfg!(feature = "mod-sharing"), deps: ["bookings"],),
        module!(name: "calendar", cat: Booking, desc: "Weekly/monthly calendar view of bookings.", enabled: cfg!(feature = "mod-calendar"), ui: "/calendar",),
        module!(name: "calendar-drag", cat: Booking, desc: "Drag-and-drop reschedule in the calendar view.", enabled: cfg!(feature = "mod-calendar-drag"), deps: ["calendar"],),
        module!(name: "favorites", cat: Booking, desc: "Pin a lot/slot as a favorite for one-tap booking.", enabled: cfg!(feature = "mod-favorites"), ui: "/favorites",),
        // ── Vehicle / Fleet ──────────────────────────────────────
        module!(name: "fleet", cat: Vehicle, desc: "Fleet admin — shared pool vehicles, utilization reports.", enabled: cfg!(feature = "mod-fleet"), ui: "/admin/fleet",),
        module!(name: "qr", cat: Vehicle, desc: "QR codes for booking confirmations + slot check-in.", enabled: cfg!(feature = "mod-qr"),),
        module!(name: "parking-pass", cat: Vehicle, desc: "Printable / digital parking pass with QR barcode.", enabled: cfg!(feature = "mod-parking-pass"),),
        module!(name: "ev-charging", cat: Experimental, desc: "EV-charging slots + charging-session metadata.", enabled: cfg!(feature = "mod-ev-charging"),),
        // ── Payment ───────────────────────────────────────────────
        module!(name: "payments", cat: Payment, desc: "Generic payment provider abstraction.", enabled: cfg!(feature = "mod-payments"),),
        module!(name: "stripe", cat: Payment, desc: "Stripe checkout + webhook integration.", enabled: cfg!(feature = "mod-stripe"), config: ["stripe_publishable_key"], deps: ["payments"],),
        module!(name: "credits", cat: Payment, desc: "Virtual credits balance — per-user monthly quota.", enabled: cfg!(feature = "mod-credits"), config: ["credits_enabled", "credits_per_booking"], ui: "/credits",),
        module!(name: "invoices", cat: Payment, desc: "Per-booking PDF invoice generation.", enabled: cfg!(feature = "mod-invoices"),),
        module!(name: "dynamic-pricing", cat: Payment, desc: "Time-of-day and occupancy-based price curves.", enabled: cfg!(feature = "mod-dynamic-pricing"),),
        // ── Admin ─────────────────────────────────────────────────
        module!(name: "rbac", cat: Admin, desc: "Role-based access control — admin, manager, user, guest.", enabled: cfg!(feature = "mod-rbac"),),
        module!(name: "sso", cat: Admin, desc: "OIDC-based single sign-on for external IdPs.", enabled: cfg!(feature = "mod-sso"),),
        module!(name: "oauth", cat: Integration, desc: "OAuth2 callback handling (Google, GitHub, Microsoft).", enabled: cfg!(feature = "mod-oauth"),),
        module!(name: "audit-export", cat: Admin, desc: "Download the audit log as CSV/JSON.", enabled: cfg!(feature = "mod-audit-export"), ui: "/admin/audit",),
        module!(name: "admin-analytics", cat: Analytics, desc: "Admin dashboards — bookings, occupancy, revenue.", enabled: cfg!(feature = "mod-admin-analytics"), ui: "/admin/analytics",),
        module!(name: "analytics", cat: Analytics, desc: "User-facing analytics — parking history trends.", enabled: cfg!(feature = "mod-analytics"),),
        module!(name: "scheduled-reports", cat: Analytics, desc: "Email cron for recurring admin reports.", enabled: cfg!(feature = "mod-scheduled-reports"),),
        module!(name: "data-import", cat: Admin, desc: "Bulk import users/vehicles/lots from CSV.", enabled: cfg!(feature = "mod-data-import"),),
        module!(name: "export", cat: Admin, desc: "GDPR + user-initiated data export.", enabled: cfg!(feature = "mod-export"),),
        module!(name: "settings", cat: Admin, desc: "Key-value runtime settings store for other modules.", enabled: cfg!(feature = "mod-settings"),),
        // ── Integration ──────────────────────────────────────────
        module!(name: "webhooks", cat: Integration, desc: "Outbound webhooks (v1 — fire and forget).", enabled: cfg!(feature = "mod-webhooks"),),
        module!(name: "webhooks-v2", cat: Integration, desc: "Outbound webhooks with retry + signed payloads.", enabled: cfg!(feature = "mod-webhooks-v2"),),
        module!(name: "graphql", cat: Integration, desc: "GraphQL read/write API alongside REST.", enabled: cfg!(feature = "mod-graphql"), ui: "/admin/graphql",),
        module!(name: "api-docs", cat: Integration, desc: "Swagger UI + OpenAPI 3.1 spec export.", enabled: cfg!(feature = "mod-api-docs"), ui: "/api/docs",),
        module!(name: "api-versioning", cat: Integration, desc: "v1/v2 API surface with deprecation headers.", enabled: cfg!(feature = "mod-api-versioning"),),
        module!(name: "ical", cat: Integration, desc: "Read-only iCal feed of bookings for calendar clients.", enabled: cfg!(feature = "mod-ical"),),
        module!(name: "websocket", cat: Integration, desc: "WebSocket broadcast for real-time occupancy + booking events.", enabled: cfg!(feature = "mod-websocket"),),
        module!(name: "widgets", cat: Integration, desc: "Embeddable widgets for external dashboards.", enabled: cfg!(feature = "mod-widgets"),),
        // ── Notification ─────────────────────────────────────────
        module!(name: "notifications", cat: Notification, desc: "In-app notification bell with per-event preferences.", enabled: cfg!(feature = "mod-notifications"), ui: "/notifications",),
        module!(name: "notification-center", cat: Notification, desc: "Grouped notification feed with mark-read.", enabled: cfg!(feature = "mod-notification-center"), deps: ["notifications"],),
        module!(name: "announcements", cat: Notification, desc: "Admin-published banner announcements.", enabled: cfg!(feature = "mod-announcements"),),
        module!(name: "email", cat: Notification, desc: "SMTP delivery of transactional emails.", enabled: cfg!(feature = "mod-email"),),
        module!(name: "email-templates", cat: Notification, desc: "Handlebars templates editable from the admin UI.", enabled: cfg!(feature = "mod-email-templates"), deps: ["email"],),
        module!(name: "push", cat: Notification, desc: "Web Push subscription + VAPID delivery.", enabled: cfg!(feature = "mod-push"),),
        // ── PWA ───────────────────────────────────────────────────
        module!(name: "pwa", cat: Experimental, desc: "Progressive Web App manifest + basic offline shell.", enabled: cfg!(feature = "mod-pwa"),),
        module!(name: "enhanced-pwa", cat: Experimental, desc: "Enhanced PWA with booking prefetch and offline data.", enabled: cfg!(feature = "mod-enhanced-pwa"), deps: ["pwa"],),
        module!(name: "mobile", cat: Experimental, desc: "Mobile-specific surfaces (install prompt, haptics).", enabled: cfg!(feature = "mod-mobile"),),
        // ── Compliance ───────────────────────────────────────────
        module!(name: "compliance", cat: Compliance, desc: "Compliance tooling — export, evidence, attestations.", enabled: cfg!(feature = "mod-compliance"),),
        module!(name: "accessible", cat: Compliance, desc: "Accessible-slots policy + user accessibility needs.", enabled: cfg!(feature = "mod-accessible"),),
        // ── Enterprise ───────────────────────────────────────────
        module!(name: "multi-tenant", cat: Enterprise, desc: "Multi-tenant isolation — per-tenant users, lots, branding.", enabled: cfg!(feature = "mod-multi-tenant"),),
        module!(name: "cost-center", cat: Enterprise, desc: "Per-cost-center reporting + allocation.", enabled: cfg!(feature = "mod-cost-center"),),
        module!(name: "themes", cat: Enterprise, desc: "Per-tenant theme / branding customization.", enabled: cfg!(feature = "mod-themes"),),
        module!(name: "plugins", cat: Enterprise, desc: "Runtime plugin loader for customer-side extensions.", enabled: cfg!(feature = "mod-plugins"), ui: "/admin/plugins",),
        module!(name: "branding", cat: Enterprise, desc: "App name + logo customization.", enabled: cfg!(feature = "mod-branding"),),
        module!(name: "translations", cat: Enterprise, desc: "Per-tenant string overrides for i18n keys.", enabled: cfg!(feature = "mod-translations"),),
        module!(name: "parking-zones", cat: Enterprise, desc: "Advanced zone management + rules.", enabled: cfg!(feature = "mod-parking-zones"),),
        // ── Experimental / Hardware ──────────────────────────────
        module!(name: "map", cat: Experimental, desc: "Map view of lots with live occupancy overlay.", enabled: cfg!(feature = "mod-map"), ui: "/map",),
        module!(name: "geofence", cat: Experimental, desc: "GPS-geofenced auto check-in.", enabled: cfg!(feature = "mod-geofence"),),
        module!(name: "visitors", cat: Experimental, desc: "Visitor badge + pre-registration.", enabled: cfg!(feature = "mod-visitors"),),
        module!(name: "maintenance", cat: Experimental, desc: "Slot maintenance windows + technician scheduling.", enabled: cfg!(feature = "mod-maintenance"),),
        module!(name: "history", cat: Experimental, desc: "Extended parking history view for users.", enabled: cfg!(feature = "mod-history"), ui: "/history",),
        module!(name: "social", cat: Experimental, desc: "Leaderboards + social sharing.", enabled: cfg!(feature = "mod-social"),),
        module!(name: "recommendations", cat: Experimental, desc: "Slot recommendations based on user history.", enabled: cfg!(feature = "mod-recommendations"),),
        module!(name: "operating-hours", cat: Experimental, desc: "Lot operating-hours enforcement.", enabled: cfg!(feature = "mod-operating-hours"),),
        module!(name: "lobby-display", cat: Experimental, desc: "Kiosk / lobby-display mode for public screens.", enabled: cfg!(feature = "mod-lobby-display"),),
        module!(name: "team", cat: Experimental, desc: "Team-level booking + fair-share policies.", enabled: cfg!(feature = "mod-team"),),
        module!(name: "setup-wizard", cat: Experimental, desc: "First-run setup wizard for admins.", enabled: cfg!(feature = "mod-setup-wizard"), ui: "/setup",),
        module!(name: "jobs", cat: Experimental, desc: "Background job scheduling (cron, intervals).", enabled: cfg!(feature = "mod-jobs"),),
    ]
}

/// `GET /api/v1/modules/info` — enriched replacement for the flat
/// Boolean-map at `/api/v1/modules`. The latter keeps working for
/// backward compatibility; this one drives the admin Modules Dashboard
/// and the Command Palette's module-command auto-registration.
pub async fn list_modules_info(
    State(_state): State<SharedState>,
) -> Json<ApiResponse<Vec<ModuleInfo>>> {
    Json(ApiResponse::success(all_modules()))
}

/// `GET /api/v1/modules/info/{name}` — detailed single-module lookup.
pub async fn get_module_info(
    State(_state): State<SharedState>,
    Path(name): Path<String>,
) -> Json<ApiResponse<ModuleInfo>> {
    match all_modules().into_iter().find(|m| m.name == name) {
        Some(info) => Json(ApiResponse::success(info)),
        None => Json(ApiResponse::error(
            "NOT_FOUND",
            format!("Unknown module '{name}'"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_entries() {
        assert!(all_modules().len() >= 50, "registry should describe all known modules");
    }

    #[test]
    fn each_module_name_is_unique() {
        let mods = all_modules();
        let mut seen = std::collections::HashSet::new();
        for m in &mods {
            assert!(
                seen.insert(m.name),
                "duplicate module name in registry: {}",
                m.name
            );
        }
    }

    #[test]
    fn dependencies_resolve_to_known_modules() {
        let mods = all_modules();
        let names: std::collections::HashSet<&str> = mods.iter().map(|m| m.name).collect();
        for m in &mods {
            for d in m.depends_on {
                assert!(
                    names.contains(d),
                    "module '{}' depends on unknown '{}'",
                    m.name,
                    d
                );
            }
        }
    }

    #[test]
    fn ui_routes_look_absolute() {
        for m in all_modules() {
            if let Some(r) = m.ui_route {
                assert!(r.starts_with('/'), "ui_route of '{}' must start with '/'", m.name);
            }
        }
    }

    #[test]
    fn config_keys_are_snake_case() {
        for m in all_modules() {
            for k in m.config_keys {
                assert!(
                    k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                    "config key '{}' of module '{}' is not snake_case",
                    k,
                    m.name
                );
            }
        }
    }
}
