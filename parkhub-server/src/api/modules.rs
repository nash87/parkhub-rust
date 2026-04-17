//! Module registry & introspection — single source of truth for every
//! optional feature ParkHub ships. Feeds both the flat `/api/v1/modules`
//! Boolean map (backwards-compat) and the enriched `ModuleInfo[]`
//! the admin Modules Dashboard and the Command Palette consume.
//!
//! Design:
//!
//! - **Declarative**: every module is one row in a `const ModuleDef`
//!   table (`REGISTRY`). Adding a module = add a row. There is no other
//!   place to touch — no scattered `register!` macros, no side-by-side
//!   descriptor/handler files, no "forgot to register" class of bugs.
//! - **Compile-time features**: `ModuleDef.feature_flag` names the
//!   `cargo` feature. `module_registry()` resolves `cfg!(feature =
//!   "mod-...")` at compile time and reports it in `enabled`.
//! - **Runtime overrides are future work** (v2). For v1 every row is
//!   marked `runtime_toggleable = false` and `runtime_enabled` == `enabled`.
//!   When a module gains a runtime gate (an admin setting that flips it
//!   off without a rebuild), flip that single row's `runtime_toggleable`
//!   to `true` and compute the setting-aware override here.
//! - **Local-only**: every string ships in the binary. No network call
//!   ever leaves the handler.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use super::SharedState;

// ═════════════════════════════════════════════════════════════════════════════
// Types
// ═════════════════════════════════════════════════════════════════════════════

/// Semantic grouping used by the admin Modules Dashboard.
///
/// Serialized as kebab-case (stable JSON, matches the kebab-case
/// module slugs and aligns with the Web UI category filter values).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleCategory {
    /// Fundamental resources — bookings, vehicles, zones.
    Core,
    /// Booking-side features — absences, recurring, guest, waitlist…
    Booking,
    /// Vehicle, fleet, QR code and parking pass features.
    Vehicle,
    /// Admin-only surfaces — RBAC, SSO, audit export, imports.
    Admin,
    /// Money flow — Stripe, credits, invoices, dynamic pricing.
    Payment,
    /// External system integrations — webhooks, calendar feeds, OAuth.
    Integration,
    /// Analytics + reporting (admin + user facing).
    Analytics,
    /// Compliance, accessibility, GDPR workflows.
    Compliance,
    /// Notification channels — push, email, in-app, announcements.
    Notification,
    /// Enterprise multi-tenant, theming, plugins, widgets.
    Enterprise,
    /// Experimental, hardware, map, geofence and edge-case features.
    Experimental,
}

/// Declarative, compile-time row in the module registry. `const`-friendly
/// so future introspection (e.g. generating docs from the table) stays
/// zero-allocation. Converted to a serializable [`ModuleInfo`] on demand.
#[derive(Debug, Clone, Copy)]
struct ModuleDef {
    /// Stable slug — matches the feature flag without the `mod-` prefix.
    name: &'static str,
    category: ModuleCategory,
    description: &'static str,
    /// Whether the binary was compiled with this module's feature flag.
    /// Evaluated at the call site with `cfg!(feature = "mod-...")`.
    enabled: bool,
    /// Admin-settings keys the module consumes. UI links each to the
    /// corresponding admin-settings section.
    config_keys: &'static [&'static str],
    /// Deep-link into the user-facing UI (e.g. `/admin/plugins`). `None`
    /// means the module has no dedicated page and surfaces only via
    /// background behavior.
    ui_route: Option<&'static str>,
    /// Other module slugs that must also be enabled for this to work.
    /// Validated at test time — broken references fail the test suite.
    depends_on: &'static [&'static str],
}

/// Runtime-serializable view of a [`ModuleDef`] with owned strings.
///
/// Two extra fields over `ModuleDef`:
///
/// - `runtime_toggleable` — `true` for modules that honour an admin
///   setting that flips them on/off without a rebuild. `false` for v1
///   across the board; flipped per-module as the runtime gate lands.
/// - `runtime_enabled` — effective enablement after applying any
///   runtime override. For v1 this always equals `enabled`; when
///   `runtime_toggleable` starts flipping to `true` the handler will
///   consult the settings store to compute this.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleInfo {
    /// Stable slug — matches the feature flag without the `mod-` prefix.
    pub name: String,
    pub category: ModuleCategory,
    pub description: String,
    /// Compile-time `cfg!(feature = "mod-...")` result.
    pub enabled: bool,
    /// When true, the module's observable behavior can be toggled at
    /// runtime via an admin setting (without a rebuild). The UI surfaces
    /// a toggle only for these.
    pub runtime_toggleable: bool,
    /// Effective enablement after applying the runtime override. Equal
    /// to `enabled` when the module is not runtime-toggleable.
    pub runtime_enabled: bool,
    /// Admin-settings keys the module consumes.
    pub config_keys: Vec<String>,
    /// Deep-link into the user-facing UI.
    pub ui_route: Option<String>,
    /// Other module slugs that must be enabled for this to work.
    pub depends_on: Vec<String>,
    /// Semver — tied to the workspace version.
    pub version: String,
}

/// Backwards-compatible envelope for `GET /api/v1/modules`.
///
/// - `modules`: the legacy flat Boolean map. Kept exactly as it was so
///   old clients and integration tests keep passing.
/// - `module_info`: the enriched `Vec<ModuleInfo>` added in T-1720.
/// - `version`: the workspace crate version.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListModulesResponse {
    /// Legacy flat feature map — `{ "bookings": true, "vehicles": true, … }`.
    pub modules: HashMap<String, bool>,
    /// Enriched per-module metadata. Order matches the registry.
    pub module_info: Vec<ModuleInfo>,
    /// Workspace crate version (semver).
    pub version: String,
}

// ═════════════════════════════════════════════════════════════════════════════
// Registry
// ═════════════════════════════════════════════════════════════════════════════

/// Convert a compile-time [`ModuleDef`] into an owned [`ModuleInfo`].
///
/// Applies the v1 policy:
/// - `runtime_toggleable = false` (no runtime override wired yet)
/// - `runtime_enabled = enabled` (same reason)
///
/// When runtime gates land (v2), the handler will look up per-module
/// settings here instead of mirroring `enabled`.
fn materialize(def: &ModuleDef) -> ModuleInfo {
    ModuleInfo {
        name: def.name.to_string(),
        category: def.category,
        description: def.description.to_string(),
        enabled: def.enabled,
        runtime_toggleable: false,
        runtime_enabled: def.enabled,
        config_keys: def.config_keys.iter().map(|s| (*s).to_string()).collect(),
        ui_route: def.ui_route.map(std::string::ToString::to_string),
        depends_on: def.depends_on.iter().map(|s| (*s).to_string()).collect(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Build the full list of modules for this build. Called per request —
/// the underlying registry is static, so this is effectively just a
/// vector-of-owned-strings allocation.
#[must_use]
pub fn module_registry() -> Vec<ModuleInfo> {
    registry_defs().iter().map(materialize).collect()
}

/// The one and only declarative registry. Every module lives here;
/// new modules are added as a single row.
///
/// Kept as a private function (rather than a `const`) because `cfg!`
/// expands to a non-const boolean expression. The function is fully
/// stateless — the compiler optimizes each `cfg!` to a constant and
/// the whole vector collapses to a set of literal bool writes.
#[allow(clippy::too_many_lines)]
fn registry_defs() -> Vec<ModuleDef> {
    vec![
        // ── Core ────────────────────────────────────────────────────────────
        ModuleDef {
            name: "bookings",
            category: ModuleCategory::Core,
            description: "Create, edit, cancel, and query reservations.",
            enabled: cfg!(feature = "mod-bookings"),
            config_keys: &[
                "require_vehicle",
                "min_booking_duration_hours",
                "license_plate_mode",
            ],
            ui_route: Some("/bookings"),
            depends_on: &[],
        },
        ModuleDef {
            name: "vehicles",
            category: ModuleCategory::Core,
            description: "User-owned vehicle records with plates and default selection.",
            enabled: cfg!(feature = "mod-vehicles"),
            config_keys: &[],
            ui_route: Some("/vehicles"),
            depends_on: &[],
        },
        ModuleDef {
            name: "zones",
            category: ModuleCategory::Core,
            description: "Organizational groupings of slots (Level A/B, EV row, visitor row).",
            enabled: cfg!(feature = "mod-zones"),
            config_keys: &[],
            ui_route: Some("/admin/zones"),
            depends_on: &[],
        },
        // ── Booking-side ────────────────────────────────────────────────────
        ModuleDef {
            name: "absences",
            category: ModuleCategory::Booking,
            description: "Home-office, vacation, sick, training — drives auto-release of booked slots.",
            enabled: cfg!(feature = "mod-absences"),
            config_keys: &["auto_release_on_absence"],
            ui_route: Some("/absences"),
            depends_on: &[],
        },
        ModuleDef {
            name: "absence-approval",
            category: ModuleCategory::Booking,
            description: "Manager approval workflow for absence requests.",
            enabled: cfg!(feature = "mod-absence-approval"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["absences"],
        },
        ModuleDef {
            name: "recurring",
            category: ModuleCategory::Booking,
            description: "Weekly / monthly recurring bookings with rule engine.",
            enabled: cfg!(feature = "mod-recurring"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "guest",
            category: ModuleCategory::Booking,
            description: "Guest passes — share a time-limited booking with an external visitor.",
            enabled: cfg!(feature = "mod-guest"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "swap",
            category: ModuleCategory::Booking,
            description: "Peer-to-peer booking swaps between users.",
            enabled: cfg!(feature = "mod-swap"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "waitlist",
            category: ModuleCategory::Booking,
            description: "Waitlist notify-on-availability.",
            enabled: cfg!(feature = "mod-waitlist"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "waitlist-ext",
            category: ModuleCategory::Booking,
            description: "Advanced waitlist — priority, expiry, multi-slot.",
            enabled: cfg!(feature = "mod-waitlist-ext"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["waitlist"],
        },
        ModuleDef {
            name: "sharing",
            category: ModuleCategory::Booking,
            description: "Shareable booking links with QR + guest registration.",
            enabled: cfg!(feature = "mod-sharing"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "calendar",
            category: ModuleCategory::Booking,
            description: "Weekly/monthly calendar view of bookings.",
            enabled: cfg!(feature = "mod-calendar"),
            config_keys: &[],
            ui_route: Some("/calendar"),
            depends_on: &[],
        },
        ModuleDef {
            name: "calendar-drag",
            category: ModuleCategory::Booking,
            description: "Drag-and-drop reschedule in the calendar view.",
            enabled: cfg!(feature = "mod-calendar-drag"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["calendar"],
        },
        ModuleDef {
            name: "favorites",
            category: ModuleCategory::Booking,
            description: "Pin a lot/slot as a favorite for one-tap booking.",
            enabled: cfg!(feature = "mod-favorites"),
            config_keys: &[],
            ui_route: Some("/favorites"),
            depends_on: &[],
        },
        // ── Vehicle / Fleet ─────────────────────────────────────────────────
        ModuleDef {
            name: "fleet",
            category: ModuleCategory::Vehicle,
            description: "Fleet admin — shared pool vehicles, utilization reports.",
            enabled: cfg!(feature = "mod-fleet"),
            config_keys: &[],
            ui_route: Some("/admin/fleet"),
            depends_on: &[],
        },
        ModuleDef {
            name: "qr",
            category: ModuleCategory::Vehicle,
            description: "QR codes for booking confirmations + slot check-in.",
            enabled: cfg!(feature = "mod-qr"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "parking-pass",
            category: ModuleCategory::Vehicle,
            description: "Printable / digital parking pass with QR barcode.",
            enabled: cfg!(feature = "mod-parking-pass"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Payment ─────────────────────────────────────────────────────────
        ModuleDef {
            name: "payments",
            category: ModuleCategory::Payment,
            description: "Generic payment provider abstraction.",
            enabled: cfg!(feature = "mod-payments"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "stripe",
            category: ModuleCategory::Payment,
            description: "Stripe checkout + webhook integration.",
            enabled: cfg!(feature = "mod-stripe"),
            config_keys: &["stripe_publishable_key"],
            ui_route: None,
            depends_on: &["payments"],
        },
        ModuleDef {
            name: "credits",
            category: ModuleCategory::Payment,
            description: "Virtual credits balance — per-user monthly quota.",
            enabled: cfg!(feature = "mod-credits"),
            config_keys: &["credits_enabled", "credits_per_booking"],
            ui_route: Some("/credits"),
            depends_on: &[],
        },
        ModuleDef {
            name: "invoices",
            category: ModuleCategory::Payment,
            description: "Per-booking PDF invoice generation.",
            enabled: cfg!(feature = "mod-invoices"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "dynamic-pricing",
            category: ModuleCategory::Payment,
            description: "Time-of-day and occupancy-based price curves.",
            enabled: cfg!(feature = "mod-dynamic-pricing"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Admin ───────────────────────────────────────────────────────────
        ModuleDef {
            name: "rbac",
            category: ModuleCategory::Admin,
            description: "Role-based access control — admin, manager, user, guest.",
            enabled: cfg!(feature = "mod-rbac"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "sso",
            category: ModuleCategory::Admin,
            description: "OIDC-based single sign-on for external IdPs.",
            enabled: cfg!(feature = "mod-sso"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "audit-export",
            category: ModuleCategory::Admin,
            description: "Download the audit log as CSV/JSON.",
            enabled: cfg!(feature = "mod-audit-export"),
            config_keys: &[],
            ui_route: Some("/admin/audit"),
            depends_on: &[],
        },
        ModuleDef {
            name: "data-import",
            category: ModuleCategory::Admin,
            description: "Bulk import users/vehicles/lots from CSV.",
            enabled: cfg!(feature = "mod-data-import"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "import",
            category: ModuleCategory::Admin,
            description: "CSV and iCal import endpoints (bulk user / absence).",
            enabled: cfg!(feature = "mod-import"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "export",
            category: ModuleCategory::Admin,
            description: "GDPR and user-initiated data export.",
            enabled: cfg!(feature = "mod-export"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "settings",
            category: ModuleCategory::Admin,
            description: "Key-value runtime settings store for other modules.",
            enabled: cfg!(feature = "mod-settings"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Analytics ───────────────────────────────────────────────────────
        ModuleDef {
            name: "admin-analytics",
            category: ModuleCategory::Analytics,
            description: "Admin dashboards — bookings, occupancy, revenue.",
            enabled: cfg!(feature = "mod-admin-analytics"),
            config_keys: &[],
            ui_route: Some("/admin/analytics"),
            depends_on: &[],
        },
        ModuleDef {
            name: "analytics",
            category: ModuleCategory::Analytics,
            description: "User-facing analytics — parking history trends.",
            enabled: cfg!(feature = "mod-analytics"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "scheduled-reports",
            category: ModuleCategory::Analytics,
            description: "Email cron for recurring admin reports.",
            enabled: cfg!(feature = "mod-scheduled-reports"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Integration ─────────────────────────────────────────────────────
        ModuleDef {
            name: "webhooks",
            category: ModuleCategory::Integration,
            description: "Outbound webhooks (v1 — fire and forget).",
            enabled: cfg!(feature = "mod-webhooks"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "webhooks-v2",
            category: ModuleCategory::Integration,
            description: "Outbound webhooks with retry + signed payloads.",
            enabled: cfg!(feature = "mod-webhooks-v2"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "graphql",
            category: ModuleCategory::Integration,
            description: "GraphQL read/write API alongside REST.",
            enabled: cfg!(feature = "mod-graphql"),
            config_keys: &[],
            ui_route: Some("/admin/graphql"),
            depends_on: &[],
        },
        ModuleDef {
            name: "api-docs",
            category: ModuleCategory::Integration,
            description: "Swagger UI + OpenAPI 3.1 spec export.",
            enabled: cfg!(feature = "mod-api-docs"),
            config_keys: &[],
            ui_route: Some("/api/docs"),
            depends_on: &[],
        },
        ModuleDef {
            name: "api-versioning",
            category: ModuleCategory::Integration,
            description: "v1/v2 API surface with deprecation headers.",
            enabled: cfg!(feature = "mod-api-versioning"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "oauth",
            category: ModuleCategory::Integration,
            description: "OAuth2 callback handling (Google, GitHub, Microsoft).",
            enabled: cfg!(feature = "mod-oauth"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "ical",
            category: ModuleCategory::Integration,
            description: "Read-only iCal feed of bookings for calendar clients.",
            enabled: cfg!(feature = "mod-ical"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "websocket",
            category: ModuleCategory::Integration,
            description: "WebSocket broadcast for real-time occupancy + booking events.",
            enabled: cfg!(feature = "mod-websocket"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Notification ────────────────────────────────────────────────────
        ModuleDef {
            name: "notifications",
            category: ModuleCategory::Notification,
            description: "In-app notification bell with per-event preferences.",
            enabled: cfg!(feature = "mod-notifications"),
            config_keys: &[],
            ui_route: Some("/notifications"),
            depends_on: &[],
        },
        ModuleDef {
            name: "notification-center",
            category: ModuleCategory::Notification,
            description: "Grouped notification feed with mark-read.",
            enabled: cfg!(feature = "mod-notification-center"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["notifications"],
        },
        ModuleDef {
            name: "announcements",
            category: ModuleCategory::Notification,
            description: "Admin-published banner announcements.",
            enabled: cfg!(feature = "mod-announcements"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "email",
            category: ModuleCategory::Notification,
            description: "SMTP delivery of transactional emails.",
            enabled: cfg!(feature = "mod-email"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "email-templates",
            category: ModuleCategory::Notification,
            description: "Handlebars templates editable from the admin UI.",
            enabled: cfg!(feature = "mod-email-templates"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["email"],
        },
        ModuleDef {
            name: "push",
            category: ModuleCategory::Notification,
            description: "Web Push subscription + VAPID delivery.",
            enabled: cfg!(feature = "mod-push"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Compliance ──────────────────────────────────────────────────────
        ModuleDef {
            name: "compliance",
            category: ModuleCategory::Compliance,
            description: "Compliance tooling — export, evidence, attestations.",
            enabled: cfg!(feature = "mod-compliance"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "accessible",
            category: ModuleCategory::Compliance,
            description: "Accessible-slots policy + user accessibility needs.",
            enabled: cfg!(feature = "mod-accessible"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Enterprise ──────────────────────────────────────────────────────
        ModuleDef {
            name: "multi-tenant",
            category: ModuleCategory::Enterprise,
            description: "Multi-tenant isolation — per-tenant users, lots, branding.",
            enabled: cfg!(feature = "mod-multi-tenant"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "cost-center",
            category: ModuleCategory::Enterprise,
            description: "Per-cost-center reporting + allocation.",
            enabled: cfg!(feature = "mod-cost-center"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "themes",
            category: ModuleCategory::Enterprise,
            description: "Per-tenant theme / branding customization.",
            enabled: cfg!(feature = "mod-themes"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "plugins",
            category: ModuleCategory::Enterprise,
            description: "Runtime plugin loader for customer-side extensions.",
            enabled: cfg!(feature = "mod-plugins"),
            config_keys: &[],
            ui_route: Some("/admin/plugins"),
            depends_on: &[],
        },
        ModuleDef {
            name: "branding",
            category: ModuleCategory::Enterprise,
            description: "App name + logo customization.",
            enabled: cfg!(feature = "mod-branding"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "translations",
            category: ModuleCategory::Enterprise,
            description: "Per-tenant string overrides for i18n keys.",
            enabled: cfg!(feature = "mod-translations"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "parking-zones",
            category: ModuleCategory::Enterprise,
            description: "Advanced zone management + rules.",
            enabled: cfg!(feature = "mod-parking-zones"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "widgets",
            category: ModuleCategory::Enterprise,
            description: "Embeddable widgets for external dashboards.",
            enabled: cfg!(feature = "mod-widgets"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        // ── Experimental / Hardware ─────────────────────────────────────────
        ModuleDef {
            name: "ev-charging",
            category: ModuleCategory::Experimental,
            description: "EV-charging slots + charging-session metadata.",
            enabled: cfg!(feature = "mod-ev-charging"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "map",
            category: ModuleCategory::Experimental,
            description: "Map view of lots with live occupancy overlay.",
            enabled: cfg!(feature = "mod-map"),
            config_keys: &[],
            ui_route: Some("/map"),
            depends_on: &[],
        },
        ModuleDef {
            name: "geofence",
            category: ModuleCategory::Experimental,
            description: "GPS-geofenced auto check-in.",
            enabled: cfg!(feature = "mod-geofence"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "visitors",
            category: ModuleCategory::Experimental,
            description: "Visitor badge + pre-registration.",
            enabled: cfg!(feature = "mod-visitors"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "maintenance",
            category: ModuleCategory::Experimental,
            description: "Slot maintenance windows + technician scheduling.",
            enabled: cfg!(feature = "mod-maintenance"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "history",
            category: ModuleCategory::Experimental,
            description: "Extended parking history view for users.",
            enabled: cfg!(feature = "mod-history"),
            config_keys: &[],
            ui_route: Some("/history"),
            depends_on: &[],
        },
        ModuleDef {
            name: "social",
            category: ModuleCategory::Experimental,
            description: "Leaderboards + social sharing.",
            enabled: cfg!(feature = "mod-social"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "recommendations",
            category: ModuleCategory::Experimental,
            description: "Slot recommendations based on user history.",
            enabled: cfg!(feature = "mod-recommendations"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "operating-hours",
            category: ModuleCategory::Experimental,
            description: "Lot operating-hours enforcement.",
            enabled: cfg!(feature = "mod-operating-hours"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "lobby-display",
            category: ModuleCategory::Experimental,
            description: "Kiosk / lobby-display mode for public screens.",
            enabled: cfg!(feature = "mod-lobby-display"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "team",
            category: ModuleCategory::Experimental,
            description: "Team-level booking + fair-share policies.",
            enabled: cfg!(feature = "mod-team"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "setup-wizard",
            category: ModuleCategory::Experimental,
            description: "First-run setup wizard for admins.",
            enabled: cfg!(feature = "mod-setup-wizard"),
            config_keys: &[],
            ui_route: Some("/setup"),
            depends_on: &[],
        },
        ModuleDef {
            name: "jobs",
            category: ModuleCategory::Experimental,
            description: "Background job scheduling (cron, intervals).",
            enabled: cfg!(feature = "mod-jobs"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "pwa",
            category: ModuleCategory::Experimental,
            description: "Progressive Web App manifest + basic offline shell.",
            enabled: cfg!(feature = "mod-pwa"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "enhanced-pwa",
            category: ModuleCategory::Experimental,
            description: "Enhanced PWA with booking prefetch and offline data.",
            enabled: cfg!(feature = "mod-enhanced-pwa"),
            config_keys: &[],
            ui_route: None,
            depends_on: &["pwa"],
        },
        ModuleDef {
            name: "mobile",
            category: ModuleCategory::Experimental,
            description: "Mobile-specific surfaces (install prompt, haptics).",
            enabled: cfg!(feature = "mod-mobile"),
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
    ]
}

// ═════════════════════════════════════════════════════════════════════════════
// Handlers
// ═════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/modules` — enriched module registry with backwards-compat envelope.
///
/// - `modules` preserves the flat `{name: bool}` map existing clients expect.
/// - `module_info` carries category, description, config keys, UI deep-link
///   and dependency metadata for the admin Modules Dashboard.
/// - `version` mirrors the workspace crate version.
///
/// Public on purpose — matches the existing `/api/v1/modules` policy.
#[utoipa::path(
    get,
    path = "/api/v1/modules",
    tag = "Public",
    summary = "List module features + enriched metadata",
    description = "Returns compile-time module enablement as both the legacy flat Boolean map and an enriched array of ModuleInfo objects (category, description, config keys, UI route, dependencies).",
    responses(
        (status = 200, description = "Module registry", body = ListModulesResponse)
    )
)]
pub async fn list_modules(State(_state): State<SharedState>) -> Json<ListModulesResponse> {
    let info = module_registry();
    let modules: HashMap<String, bool> = info.iter().map(|m| (m.name.clone(), m.enabled)).collect();
    Json(ListModulesResponse {
        modules,
        module_info: info,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// `GET /api/v1/modules/{name}` — single-module detail.
///
/// Returns 404 if the slug is not in the registry.
#[utoipa::path(
    get,
    path = "/api/v1/modules/{name}",
    tag = "Public",
    summary = "Get a single module by slug",
    description = "Returns enriched ModuleInfo for one module, keyed by its stable slug (matches the feature flag without the `mod-` prefix).",
    params(("name" = String, Path, description = "Module slug (e.g. 'bookings', 'admin-analytics')")),
    responses(
        (status = 200, description = "Module metadata", body = ModuleInfo),
        (status = 404, description = "Unknown module slug")
    )
)]
pub async fn get_module(
    State(_state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<ModuleInfo>, StatusCode> {
    module_registry()
        .into_iter()
        .find(|m| m.name == name)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every `ModuleCategory` variant should have at least one module assigned.
    /// Catches registry drift where we introduce a category and forget to
    /// populate it (or vice versa).
    #[test]
    fn test_all_categories_represented() {
        let registry = module_registry();
        let seen: HashSet<ModuleCategory> = registry.iter().map(|m| m.category).collect();

        let all = [
            ModuleCategory::Core,
            ModuleCategory::Booking,
            ModuleCategory::Vehicle,
            ModuleCategory::Admin,
            ModuleCategory::Payment,
            ModuleCategory::Integration,
            ModuleCategory::Analytics,
            ModuleCategory::Compliance,
            ModuleCategory::Notification,
            ModuleCategory::Enterprise,
            ModuleCategory::Experimental,
        ];
        for cat in &all {
            assert!(
                seen.contains(cat),
                "Category {cat:?} has no modules — add at least one row or drop the variant"
            );
        }
    }

    /// Spot-check that `enabled` actually reflects the cargo feature — not a
    /// hard-coded `true`/`false`. We pick three modules with diverse default-
    /// feature membership and compare against `cfg!`.
    #[test]
    fn test_compile_time_feature_detection() {
        let registry = module_registry();

        for (slug, expected) in [
            ("bookings", cfg!(feature = "mod-bookings")),
            ("vehicles", cfg!(feature = "mod-vehicles")),
            ("plugins", cfg!(feature = "mod-plugins")),
        ] {
            let m = registry
                .iter()
                .find(|m| m.name == slug)
                .unwrap_or_else(|| panic!("registry missing module '{slug}'"));
            assert_eq!(
                m.enabled, expected,
                "module '{slug}' enabled={} but cfg! reports {}",
                m.enabled, expected
            );
        }
    }

    /// Every `depends_on` entry must resolve to a real module in the registry.
    /// Catches typos + stale references.
    #[test]
    fn test_depends_on_references_real_modules() {
        let registry = module_registry();
        let names: HashSet<&str> = registry.iter().map(|m| m.name.as_str()).collect();
        for m in &registry {
            for d in &m.depends_on {
                assert!(
                    names.contains(d.as_str()),
                    "module '{}' depends on unknown module '{}'",
                    m.name,
                    d
                );
            }
        }
    }

    /// No two rows may share a slug — we use slugs as stable API keys.
    #[test]
    fn test_module_names_unique() {
        let registry = module_registry();
        let mut seen = HashSet::new();
        for m in &registry {
            assert!(
                seen.insert(m.name.clone()),
                "duplicate module slug in registry: '{}'",
                m.name
            );
        }
    }

    /// Deep-link routes must be absolute — the React router mounts at `/` and
    /// relative `ui_route`s would navigate unpredictably.
    #[test]
    fn test_ui_routes_start_with_slash() {
        for m in module_registry() {
            if let Some(route) = &m.ui_route {
                assert!(
                    route.starts_with('/'),
                    "ui_route of '{}' must start with '/' (got {route:?})",
                    m.name
                );
            }
        }
    }

    /// v1 invariant: every module is marked `runtime_toggleable = false`
    /// and `runtime_enabled == enabled`. When a module gets a runtime gate,
    /// flip its row individually AND update this test so it asserts the new
    /// state for every other row.
    #[test]
    fn test_runtime_defaults_v1() {
        for m in module_registry() {
            assert!(
                !m.runtime_toggleable,
                "module '{}' must start with runtime_toggleable=false in v1",
                m.name
            );
            assert_eq!(
                m.runtime_enabled, m.enabled,
                "module '{}' must have runtime_enabled == enabled in v1",
                m.name
            );
        }
    }

    /// Every config key must be snake_case (matches the admin settings store).
    #[test]
    fn test_config_keys_are_snake_case() {
        for m in module_registry() {
            for k in &m.config_keys {
                assert!(
                    k.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                    "config key '{k}' of module '{}' is not snake_case",
                    m.name
                );
            }
        }
    }

    /// Handler shape: calling `list_modules` should return the full envelope
    /// with both the legacy Boolean map and the enriched array populated.
    ///
    /// We build a minimal `AppState` (the handler never touches it) using the
    /// same scaffolding the rest of the server-side test modules use.
    #[tokio::test]
    async fn test_list_modules_handler_shape() {
        use crate::AppState;
        use crate::config::ServerConfig;
        use crate::db::{Database, DatabaseConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let dir = tempfile::tempdir().expect("tempdir");
        let db = Database::open(&DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        })
        .expect("open test db");

        let state = Arc::new(RwLock::new(AppState {
            config: ServerConfig::default(),
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
            revocation_store: crate::jwt::TokenRevocationList::new(),
        }));

        let Json(response) = list_modules(State(state)).await;

        // Registry has a healthy number of modules.
        assert!(
            response.module_info.len() >= 60,
            "expected at least 60 modules, got {}",
            response.module_info.len()
        );

        // Legacy map and enriched array carry the same entries.
        assert_eq!(response.modules.len(), response.module_info.len());
        for m in &response.module_info {
            assert_eq!(
                response.modules.get(&m.name).copied(),
                Some(m.enabled),
                "modules[{}] should mirror module_info[].enabled",
                m.name
            );
        }

        // Version is the workspace version.
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    /// `get_module` returns 404 on unknown slugs.
    #[tokio::test]
    async fn test_get_module_unknown_returns_404() {
        use crate::AppState;
        use crate::config::ServerConfig;
        use crate::db::{Database, DatabaseConfig};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let dir = tempfile::tempdir().expect("tempdir");
        let db = Database::open(&DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        })
        .expect("open test db");

        let state = Arc::new(RwLock::new(AppState {
            config: ServerConfig::default(),
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
            revocation_store: crate::jwt::TokenRevocationList::new(),
        }));

        let result = get_module(State(state), Path("does-not-exist".to_string())).await;
        assert!(matches!(result, Err(StatusCode::NOT_FOUND)));
    }

    /// `ModuleCategory` serializes as kebab-case so the JSON matches the
    /// module slug style and the frontend category filter values.
    #[test]
    fn test_category_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&ModuleCategory::Core).unwrap(),
            "\"core\""
        );
        assert_eq!(
            serde_json::to_string(&ModuleCategory::Experimental).unwrap(),
            "\"experimental\""
        );
        assert_eq!(
            serde_json::to_string(&ModuleCategory::Notification).unwrap(),
            "\"notification\""
        );
    }
}
