//! Module registry & introspection — single source of truth for every
//! optional feature ParkHub ships. Feeds both the flat `/api/v1/modules`
//! Boolean map (backwards-compat) and the enriched `ModuleInfo[]`
//! the admin Modules Dashboard and the Command Palette consume.
//!
//! Design:
//!
//! - **Declarative**: every module is one row in the `ModuleDef` table
//!   (`registry_defs`). Adding a module = add a row. There is no other
//!   place to touch — no scattered `register!` macros, no side-by-side
//!   descriptor/handler files, no "forgot to register" class of bugs.
//! - **Compile-time features**: `ModuleDef.enabled` resolves
//!   `cfg!(feature = "mod-...")` at compile time and is reported as
//!   `enabled`.
//! - **Runtime overrides (v2)**: modules marked `runtime_toggleable =
//!   true` honour an admin setting `module.{name}.runtime_enabled`
//!   (`"true"|"false"`) that flips the effective `runtime_enabled`
//!   without a rebuild. The override is applied only to rows that
//!   opted in — security-sensitive modules (payments, RBAC, SSO, etc.)
//!   keep `runtime_toggleable = false` and are never flippable via the
//!   API, only through a rebuild.
//! - **Audited writes**: every successful runtime toggle emits a
//!   [`crate::audit::AuditEventType::ConfigChanged`] entry carrying the
//!   caller's user_id, the module slug, and the new state.
//! - **Middleware gate**: routes owned by a runtime-toggleable module
//!   can be wrapped in [`module_gate`], which short-circuits to `404
//!   NOT_FOUND` when the module is currently disabled at runtime. A
//!   disabled module "doesn't exist" from the client's point of view —
//!   same shape as a feature that was never compiled in.
//! - **Local-only**: every string ships in the binary. No network call
//!   ever leaves the handler.

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use parkhub_common::ApiResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState, check_admin};

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
    /// Whether the module honours a runtime admin-setting override.
    ///
    /// `true` for modules that can be flipped off at runtime without a
    /// rebuild (low blast-radius, safe-to-toggle surfaces — UI widgets,
    /// display-only integrations, experimental features).
    ///
    /// `false` for modules whose runtime toggle would be a foot-gun
    /// (payments, RBAC, SSO, audit export, compliance, multi-tenant —
    /// anything that guards money, identity, or the audit trail). These
    /// can only be disabled by a rebuild, matching their security
    /// weight.
    runtime_toggleable: bool,
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
/// Shape used by `GET /api/v1/modules`, `GET /api/v1/modules/{name}`
/// and the `PATCH /api/v1/admin/modules/{name}` response body.
///
/// - `runtime_toggleable` — `true` for modules that honour an admin
///   setting that flips them on/off without a rebuild. Fifteen safe-to-
///   toggle rows are marked true; security-sensitive modules keep it
///   `false`.
/// - `runtime_enabled` — effective enablement after applying any
///   runtime override. For rows with `runtime_toggleable = false` this
///   always equals `enabled`. For toggleable rows, the handler reads
///   `module.{name}.runtime_enabled` from the settings store and
///   overrides this field accordingly (see [`module_registry`]).
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

/// Build the admin-settings key that stores a module's runtime override.
///
/// Shape: `module.{slug}.runtime_enabled`. Values are `"true"` or
/// `"false"` (stringly-typed to match the existing settings store which
/// is a `String -> String` map).
#[must_use]
pub fn runtime_enabled_setting_key(module_name: &str) -> String {
    format!("module.{module_name}.runtime_enabled")
}

/// Convert a compile-time [`ModuleDef`] into an owned [`ModuleInfo`]
/// **without** applying any runtime setting overrides.
///
/// This is the pure compile-time view — used by tests, by callers that
/// don't need the admin-settings override, and as the starting point
/// for [`module_registry`] before it applies overrides.
fn materialize(def: &ModuleDef) -> ModuleInfo {
    ModuleInfo {
        name: def.name.to_string(),
        category: def.category,
        description: def.description.to_string(),
        enabled: def.enabled,
        runtime_toggleable: def.runtime_toggleable,
        runtime_enabled: def.enabled,
        config_keys: def.config_keys.iter().map(|s| (*s).to_string()).collect(),
        ui_route: def.ui_route.map(std::string::ToString::to_string),
        depends_on: def.depends_on.iter().map(|s| (*s).to_string()).collect(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Pure compile-time view of the registry — no settings lookup, no DB
/// access. Useful for tests and for handlers that only need to check
/// `enabled` / metadata, not the effective runtime state.
#[must_use]
pub fn module_registry_static() -> Vec<ModuleInfo> {
    registry_defs().iter().map(materialize).collect()
}

/// Build the full list of modules with per-module **runtime overrides**
/// applied.
///
/// For every row with `runtime_toggleable = true`, reads the setting
/// `module.{name}.runtime_enabled` from [`crate::db::Database`] and uses
/// it to compute `runtime_enabled`:
///
/// - `"true"` → `runtime_enabled = true` (but only if `enabled` is
///   also true — a compile-time-disabled module can never be enabled at
///   runtime, the code simply isn't there).
/// - `"false"` → `runtime_enabled = false`.
/// - setting missing / invalid → `runtime_enabled = enabled` (default
///   to compile-time state).
///
/// Rows with `runtime_toggleable = false` always have
/// `runtime_enabled == enabled`, regardless of any setting. A stale or
/// malicious setting on a non-toggleable module has zero effect.
pub async fn module_registry(db: &crate::db::Database) -> Vec<ModuleInfo> {
    let mut out = module_registry_static();
    for info in &mut out {
        if !info.runtime_toggleable {
            continue;
        }
        let key = runtime_enabled_setting_key(&info.name);
        if let Ok(Some(value)) = db.get_setting(&key).await {
            info.runtime_enabled = match value.as_str() {
                "true" => info.enabled,
                "false" => false,
                // Any other value is treated as "not set" → fall back to
                // the compile-time default (already materialised above).
                _ => info.runtime_enabled,
            };
        }
    }
    out
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
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/vehicles"),
            depends_on: &[],
        },
        ModuleDef {
            name: "zones",
            category: ModuleCategory::Core,
            description: "Organizational groupings of slots (Level A/B, EV row, visitor row).",
            enabled: cfg!(feature = "mod-zones"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &["auto_release_on_absence"],
            ui_route: Some("/absences"),
            depends_on: &[],
        },
        ModuleDef {
            name: "absence-approval",
            category: ModuleCategory::Booking,
            description: "Manager approval workflow for absence requests.",
            enabled: cfg!(feature = "mod-absence-approval"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["absences"],
        },
        ModuleDef {
            name: "recurring",
            category: ModuleCategory::Booking,
            description: "Weekly / monthly recurring bookings with rule engine.",
            enabled: cfg!(feature = "mod-recurring"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "guest",
            category: ModuleCategory::Booking,
            description: "Guest passes — share a time-limited booking with an external visitor.",
            enabled: cfg!(feature = "mod-guest"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "swap",
            category: ModuleCategory::Booking,
            description: "Peer-to-peer booking swaps between users.",
            enabled: cfg!(feature = "mod-swap"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "waitlist",
            category: ModuleCategory::Booking,
            description: "Waitlist notify-on-availability.",
            enabled: cfg!(feature = "mod-waitlist"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "waitlist-ext",
            category: ModuleCategory::Booking,
            description: "Advanced waitlist — priority, expiry, multi-slot.",
            enabled: cfg!(feature = "mod-waitlist-ext"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["waitlist"],
        },
        ModuleDef {
            name: "sharing",
            category: ModuleCategory::Booking,
            description: "Shareable booking links with QR + guest registration.",
            enabled: cfg!(feature = "mod-sharing"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["bookings"],
        },
        ModuleDef {
            name: "calendar",
            category: ModuleCategory::Booking,
            description: "Weekly/monthly calendar view of bookings.",
            enabled: cfg!(feature = "mod-calendar"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/calendar"),
            depends_on: &[],
        },
        ModuleDef {
            name: "calendar-drag",
            category: ModuleCategory::Booking,
            description: "Drag-and-drop reschedule in the calendar view.",
            enabled: cfg!(feature = "mod-calendar-drag"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &["calendar"],
        },
        ModuleDef {
            name: "favorites",
            category: ModuleCategory::Booking,
            description: "Pin a lot/slot as a favorite for one-tap booking.",
            enabled: cfg!(feature = "mod-favorites"),
            runtime_toggleable: true,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/admin/fleet"),
            depends_on: &[],
        },
        ModuleDef {
            name: "qr",
            category: ModuleCategory::Vehicle,
            description: "QR codes for booking confirmations + slot check-in.",
            enabled: cfg!(feature = "mod-qr"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "parking-pass",
            category: ModuleCategory::Vehicle,
            description: "Printable / digital parking pass with QR barcode.",
            enabled: cfg!(feature = "mod-parking-pass"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "stripe",
            category: ModuleCategory::Payment,
            description: "Stripe checkout + webhook integration.",
            enabled: cfg!(feature = "mod-stripe"),
            runtime_toggleable: false,
            config_keys: &["stripe_publishable_key"],
            ui_route: None,
            depends_on: &["payments"],
        },
        ModuleDef {
            name: "credits",
            category: ModuleCategory::Payment,
            description: "Virtual credits balance — per-user monthly quota.",
            enabled: cfg!(feature = "mod-credits"),
            runtime_toggleable: false,
            config_keys: &["credits_enabled", "credits_per_booking"],
            ui_route: Some("/credits"),
            depends_on: &[],
        },
        ModuleDef {
            name: "invoices",
            category: ModuleCategory::Payment,
            description: "Per-booking PDF invoice generation.",
            enabled: cfg!(feature = "mod-invoices"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "dynamic-pricing",
            category: ModuleCategory::Payment,
            description: "Time-of-day and occupancy-based price curves.",
            enabled: cfg!(feature = "mod-dynamic-pricing"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "sso",
            category: ModuleCategory::Admin,
            description: "OIDC-based single sign-on for external IdPs.",
            enabled: cfg!(feature = "mod-sso"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "audit-export",
            category: ModuleCategory::Admin,
            description: "Download the audit log as CSV/JSON.",
            enabled: cfg!(feature = "mod-audit-export"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/admin/audit"),
            depends_on: &[],
        },
        ModuleDef {
            name: "data-import",
            category: ModuleCategory::Admin,
            description: "Bulk import users/vehicles/lots from CSV.",
            enabled: cfg!(feature = "mod-data-import"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "import",
            category: ModuleCategory::Admin,
            description: "CSV and iCal import endpoints (bulk user / absence).",
            enabled: cfg!(feature = "mod-import"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "export",
            category: ModuleCategory::Admin,
            description: "GDPR and user-initiated data export.",
            enabled: cfg!(feature = "mod-export"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "settings",
            category: ModuleCategory::Admin,
            description: "Key-value runtime settings store for other modules.",
            enabled: cfg!(feature = "mod-settings"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/admin/analytics"),
            depends_on: &[],
        },
        ModuleDef {
            name: "analytics",
            category: ModuleCategory::Analytics,
            description: "User-facing analytics — parking history trends.",
            enabled: cfg!(feature = "mod-analytics"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "scheduled-reports",
            category: ModuleCategory::Analytics,
            description: "Email cron for recurring admin reports.",
            enabled: cfg!(feature = "mod-scheduled-reports"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "webhooks-v2",
            category: ModuleCategory::Integration,
            description: "Outbound webhooks with retry + signed payloads.",
            enabled: cfg!(feature = "mod-webhooks-v2"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "graphql",
            category: ModuleCategory::Integration,
            description: "GraphQL read/write API alongside REST.",
            enabled: cfg!(feature = "mod-graphql"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: Some("/admin/graphql"),
            depends_on: &[],
        },
        ModuleDef {
            name: "api-docs",
            category: ModuleCategory::Integration,
            description: "Swagger UI + OpenAPI 3.1 spec export.",
            enabled: cfg!(feature = "mod-api-docs"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: Some("/api/docs"),
            depends_on: &[],
        },
        ModuleDef {
            name: "api-versioning",
            category: ModuleCategory::Integration,
            description: "v1/v2 API surface with deprecation headers.",
            enabled: cfg!(feature = "mod-api-versioning"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "oauth",
            category: ModuleCategory::Integration,
            description: "OAuth2 callback handling (Google, GitHub, Microsoft).",
            enabled: cfg!(feature = "mod-oauth"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "ical",
            category: ModuleCategory::Integration,
            description: "Read-only iCal feed of bookings for calendar clients.",
            enabled: cfg!(feature = "mod-ical"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "websocket",
            category: ModuleCategory::Integration,
            description: "WebSocket broadcast for real-time occupancy + booking events.",
            enabled: cfg!(feature = "mod-websocket"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/notifications"),
            depends_on: &[],
        },
        ModuleDef {
            name: "notification-center",
            category: ModuleCategory::Notification,
            description: "Grouped notification feed with mark-read.",
            enabled: cfg!(feature = "mod-notification-center"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["notifications"],
        },
        ModuleDef {
            name: "announcements",
            category: ModuleCategory::Notification,
            description: "Admin-published banner announcements.",
            enabled: cfg!(feature = "mod-announcements"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "email",
            category: ModuleCategory::Notification,
            description: "SMTP delivery of transactional emails.",
            enabled: cfg!(feature = "mod-email"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "email-templates",
            category: ModuleCategory::Notification,
            description: "Handlebars templates editable from the admin UI.",
            enabled: cfg!(feature = "mod-email-templates"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["email"],
        },
        ModuleDef {
            name: "push",
            category: ModuleCategory::Notification,
            description: "Web Push subscription + VAPID delivery.",
            enabled: cfg!(feature = "mod-push"),
            runtime_toggleable: false,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "accessible",
            category: ModuleCategory::Compliance,
            description: "Accessible-slots policy + user accessibility needs.",
            enabled: cfg!(feature = "mod-accessible"),
            runtime_toggleable: true,
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
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "cost-center",
            category: ModuleCategory::Enterprise,
            description: "Per-cost-center reporting + allocation.",
            enabled: cfg!(feature = "mod-cost-center"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "themes",
            category: ModuleCategory::Enterprise,
            description: "Per-tenant theme / branding customization.",
            enabled: cfg!(feature = "mod-themes"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "plugins",
            category: ModuleCategory::Enterprise,
            description: "Runtime plugin loader for customer-side extensions.",
            enabled: cfg!(feature = "mod-plugins"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/admin/plugins"),
            depends_on: &[],
        },
        ModuleDef {
            name: "branding",
            category: ModuleCategory::Enterprise,
            description: "App name + logo customization.",
            enabled: cfg!(feature = "mod-branding"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "translations",
            category: ModuleCategory::Enterprise,
            description: "Per-tenant string overrides for i18n keys.",
            enabled: cfg!(feature = "mod-translations"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "parking-zones",
            category: ModuleCategory::Enterprise,
            description: "Advanced zone management + rules.",
            enabled: cfg!(feature = "mod-parking-zones"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "widgets",
            category: ModuleCategory::Enterprise,
            description: "Embeddable widgets for external dashboards.",
            enabled: cfg!(feature = "mod-widgets"),
            runtime_toggleable: true,
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
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "map",
            category: ModuleCategory::Experimental,
            description: "Map view of lots with live occupancy overlay.",
            enabled: cfg!(feature = "mod-map"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: Some("/map"),
            depends_on: &[],
        },
        ModuleDef {
            name: "geofence",
            category: ModuleCategory::Experimental,
            description: "GPS-geofenced auto check-in.",
            enabled: cfg!(feature = "mod-geofence"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "visitors",
            category: ModuleCategory::Experimental,
            description: "Visitor badge + pre-registration.",
            enabled: cfg!(feature = "mod-visitors"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "maintenance",
            category: ModuleCategory::Experimental,
            description: "Slot maintenance windows + technician scheduling.",
            enabled: cfg!(feature = "mod-maintenance"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "history",
            category: ModuleCategory::Experimental,
            description: "Extended parking history view for users.",
            enabled: cfg!(feature = "mod-history"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: Some("/history"),
            depends_on: &[],
        },
        ModuleDef {
            name: "social",
            category: ModuleCategory::Experimental,
            description: "Leaderboards + social sharing.",
            enabled: cfg!(feature = "mod-social"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "recommendations",
            category: ModuleCategory::Experimental,
            description: "Slot recommendations based on user history.",
            enabled: cfg!(feature = "mod-recommendations"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "operating-hours",
            category: ModuleCategory::Experimental,
            description: "Lot operating-hours enforcement.",
            enabled: cfg!(feature = "mod-operating-hours"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "lobby-display",
            category: ModuleCategory::Experimental,
            description: "Kiosk / lobby-display mode for public screens.",
            enabled: cfg!(feature = "mod-lobby-display"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "team",
            category: ModuleCategory::Experimental,
            description: "Team-level booking + fair-share policies.",
            enabled: cfg!(feature = "mod-team"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "setup-wizard",
            category: ModuleCategory::Experimental,
            description: "First-run setup wizard for admins.",
            enabled: cfg!(feature = "mod-setup-wizard"),
            runtime_toggleable: true,
            config_keys: &[],
            ui_route: Some("/setup"),
            depends_on: &[],
        },
        ModuleDef {
            name: "jobs",
            category: ModuleCategory::Experimental,
            description: "Background job scheduling (cron, intervals).",
            enabled: cfg!(feature = "mod-jobs"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "pwa",
            category: ModuleCategory::Experimental,
            description: "Progressive Web App manifest + basic offline shell.",
            enabled: cfg!(feature = "mod-pwa"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &[],
        },
        ModuleDef {
            name: "enhanced-pwa",
            category: ModuleCategory::Experimental,
            description: "Enhanced PWA with booking prefetch and offline data.",
            enabled: cfg!(feature = "mod-enhanced-pwa"),
            runtime_toggleable: false,
            config_keys: &[],
            ui_route: None,
            depends_on: &["pwa"],
        },
        ModuleDef {
            name: "mobile",
            category: ModuleCategory::Experimental,
            description: "Mobile-specific surfaces (install prompt, haptics).",
            enabled: cfg!(feature = "mod-mobile"),
            runtime_toggleable: false,
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
/// - `module_info` carries category, description, config keys, UI deep-link,
///   dependency metadata, `runtime_toggleable`, and the effective
///   `runtime_enabled` state (after admin-setting override has been
///   applied).
/// - `version` mirrors the workspace crate version.
///
/// Public on purpose — matches the existing `/api/v1/modules` policy.
/// Only compile-time enablement + the effective runtime state are leaked;
/// no settings values, no config payloads.
#[utoipa::path(
    get,
    path = "/api/v1/modules",
    tag = "Public",
    summary = "List module features + enriched metadata",
    description = "Returns compile-time module enablement as both the legacy flat Boolean map and an enriched array of ModuleInfo objects (category, description, config keys, UI route, dependencies, runtime_toggleable, runtime_enabled).",
    responses(
        (status = 200, description = "Module registry", body = ListModulesResponse)
    )
)]
pub async fn list_modules(State(state): State<SharedState>) -> Json<ListModulesResponse> {
    let state_guard = state.read().await;
    let info = module_registry(&state_guard.db).await;
    drop(state_guard);
    let modules: HashMap<String, bool> = info
        .iter()
        .map(|m| (m.name.clone(), m.runtime_enabled))
        .collect();
    Json(ListModulesResponse {
        modules,
        module_info: info,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// `GET /api/v1/modules/{name}` — single-module detail.
///
/// Returns 404 if the slug is not in the registry. Runtime overrides are
/// applied — `runtime_enabled` reflects the current admin-setting state.
#[utoipa::path(
    get,
    path = "/api/v1/modules/{name}",
    tag = "Public",
    summary = "Get a single module by slug",
    description = "Returns enriched ModuleInfo for one module, keyed by its stable slug (matches the feature flag without the `mod-` prefix). Runtime setting overrides are applied.",
    params(("name" = String, Path, description = "Module slug (e.g. 'bookings', 'admin-analytics')")),
    responses(
        (status = 200, description = "Module metadata", body = ModuleInfo),
        (status = 404, description = "Unknown module slug")
    )
)]
pub async fn get_module(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> Result<Json<ModuleInfo>, StatusCode> {
    let state_guard = state.read().await;
    let info = module_registry(&state_guard.db).await;
    drop(state_guard);
    info.into_iter()
        .find(|m| m.name == name)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// ═════════════════════════════════════════════════════════════════════════════
// Runtime toggle — PATCH /api/v1/admin/modules/{name}
// ═════════════════════════════════════════════════════════════════════════════

/// Request body for the runtime-toggle endpoint.
///
/// A deliberately minimal shape — only `runtime_enabled` is settable
/// from the admin UI. `enabled` stays compile-time-only (matches the
/// underlying cargo feature flag and therefore a rebuild).
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpdateModuleRequest {
    /// New effective runtime enablement. Persisted to admin setting
    /// `module.{name}.runtime_enabled`.
    pub runtime_enabled: bool,
}

/// `PATCH /api/v1/admin/modules/{name}` — flip a module's runtime state.
///
/// Admin only. Writes the admin setting `module.{name}.runtime_enabled`
/// and returns the updated [`ModuleInfo`].
///
/// Error map:
/// - `400 BAD_REQUEST` — unknown module slug.
/// - `409 CONFLICT` — the module is marked
///   `runtime_toggleable = false` and cannot be flipped at runtime
///   (security-sensitive modules — payments, RBAC, SSO, audit export,
///   compliance, multi-tenant). Rebuild the binary without the feature
///   instead.
///
/// Every successful toggle emits a `ConfigChanged` audit entry with the
/// caller's user_id, the target module slug, and the new state.
#[utoipa::path(
    patch,
    path = "/api/v1/admin/modules/{name}",
    tag = "Admin",
    summary = "Toggle a module's runtime state",
    description = "Admin-only. Flips a runtime-toggleable module on/off via admin setting `module.{name}.runtime_enabled`. Returns 400 for unknown slug and 409 when the module is marked runtime_toggleable=false (security-sensitive — cannot be disabled without a rebuild). Audit-logged.",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Module slug (kebab-case)")),
    request_body = UpdateModuleRequest,
    responses(
        (status = 200, description = "Updated module state", body = ModuleInfo),
        (status = 400, description = "Unknown module slug"),
        (status = 403, description = "Admin access required"),
        (status = 409, description = "Module is not runtime_toggleable")
    )
)]
pub async fn patch_admin_module(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Path(name): Path<String>,
    Json(body): Json<UpdateModuleRequest>,
) -> (StatusCode, Json<ApiResponse<ModuleInfo>>) {
    let state_guard = state.read().await;

    // Admin guard (defense-in-depth on top of the admin_middleware layer).
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    // Resolve the module, defaulting to the compile-time-only view so we
    // can return 400 for unknown slugs without needing the full override
    // lookup.
    let Some(current) = module_registry_static()
        .into_iter()
        .find(|m| m.name == name)
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "UNKNOWN_MODULE",
                format!("Unknown module slug: {name}"),
            )),
        );
    };

    if !current.runtime_toggleable {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "NOT_RUNTIME_TOGGLEABLE",
                format!(
                    "Module '{name}' cannot be toggled at runtime — rebuild the binary without its cargo feature to disable it."
                ),
            )),
        );
    }

    let key = runtime_enabled_setting_key(&name);
    let value = if body.runtime_enabled {
        "true"
    } else {
        "false"
    };
    if let Err(e) = state_guard.db.set_setting(&key, value).await {
        tracing::error!(module = %name, error = %e, "Failed to persist module runtime toggle");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save runtime toggle",
            )),
        );
    }

    // Audit log — ConfigChanged captures the caller + the exact change.
    if state_guard.config.audit_logging_enabled {
        let admin_username = state_guard
            .db
            .get_user(&auth_user.user_id.to_string())
            .await
            .ok()
            .flatten()
            .map(|u| u.username)
            .unwrap_or_default();

        let entry = AuditEntry::new(AuditEventType::ConfigChanged)
            .user(auth_user.user_id, &admin_username)
            .resource("module", &name)
            .details(serde_json::json!({
                "setting_key": key,
                "runtime_enabled": body.runtime_enabled,
            }))
            .log();
        entry.persist(&state_guard.db).await;
    }

    // Re-read the registry so the response reflects the freshly-written
    // override.
    let info = module_registry(&state_guard.db).await;
    drop(state_guard);
    let Some(updated) = info.into_iter().find(|m| m.name == name) else {
        // Registry must contain `name` — we already validated above. If
        // this ever fires, the registry mutated mid-request; return 500
        // rather than invent a value.
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Module disappeared after toggle",
            )),
        );
    };

    (StatusCode::OK, Json(ApiResponse::success(updated)))
}

// ═════════════════════════════════════════════════════════════════════════════
// Middleware — module_gate
// ═════════════════════════════════════════════════════════════════════════════

/// Pairs a runtime-toggleable module with the route prefixes it owns.
///
/// When the module is disabled at runtime, any request whose path
/// starts with one of these prefixes is short-circuited by
/// [`module_gate`] with a `404 NOT_FOUND` — as if the feature was never
/// compiled in.
///
/// v2 policy: only the small set of low-risk modules below is wired
/// into the gate. Route enumeration for the remaining 10
/// runtime-toggleable modules can come in v3 as the surface stabilises.
/// Adding a module here is the final step that turns its admin-settings
/// toggle into a real runtime kill-switch at the HTTP layer.
pub const MODULE_ROUTES: &[(&str, &[&str])] = &[
    // Map view: public lot markers endpoint.
    ("map", &["/api/v1/lots/map"]),
    // GraphQL: playground + schema (public) and execute (protected) all
    // share the `/api/v1/graphql` prefix.
    ("graphql", &["/api/v1/graphql"]),
    // API docs: interactive UI + JSON/Postman exports.
    ("api-docs", &["/api/v1/docs"]),
    // Public "active announcements" list. Admin CRUD lives under
    // /api/v1/admin/announcements and is intentionally kept reachable
    // so admins can re-enable the module via the dashboard even when
    // the public surface is turned off.
    ("announcements", &["/api/v1/announcements"]),
    // Personal favorites (pin a slot).
    ("favorites", &["/api/v1/user/favorites"]),
];

/// Look up the module slug that owns a given request path, if any.
///
/// Returns the first module whose route-prefix table matches `path`.
/// Used by [`module_gate`] — exposed for tests.
#[must_use]
pub fn module_for_path(path: &str) -> Option<&'static str> {
    for (module, prefixes) in MODULE_ROUTES {
        for prefix in *prefixes {
            if path == *prefix || path.starts_with(&format!("{prefix}/")) {
                return Some(*module);
            }
        }
    }
    None
}

/// Axum middleware that short-circuits requests to runtime-disabled modules.
///
/// Flow:
/// 1. Extract the request path.
/// 2. Find the owning module (via [`MODULE_ROUTES`]). No owner → pass.
/// 3. Look up the module in the registry. Non-toggleable → pass (this
///    is a belt-and-suspenders check; non-toggleable modules should not
///    have an entry in [`MODULE_ROUTES`] in the first place).
/// 4. If `runtime_enabled = false` → `404 NOT_FOUND`. Otherwise pass.
///
/// A disabled module is indistinguishable from a feature that was never
/// compiled in — same status code, same error body. This keeps the
/// failure mode uniform across the two disable paths (feature flag vs.
/// runtime toggle).
pub async fn module_gate(
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let Some(module_name) = module_for_path(&path) else {
        return next.run(request).await;
    };

    let state_guard = state.read().await;
    let registry = module_registry(&state_guard.db).await;
    drop(state_guard);

    let Some(info) = registry.into_iter().find(|m| m.name == module_name) else {
        // Unknown module name in MODULE_ROUTES — treat as a config bug
        // but let the request through rather than breaking traffic.
        tracing::warn!(
            module = module_name,
            path = %path,
            "module_gate: route references unknown module"
        );
        return next.run(request).await;
    };

    if info.runtime_toggleable && !info.runtime_enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error(
                "MODULE_DISABLED",
                format!("Module '{module_name}' is disabled"),
            )),
        )
            .into_response();
    }

    next.run(request).await
}

// ═════════════════════════════════════════════════════════════════════════════
// Tests
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Build a minimal `AppState` for handler + middleware tests. Opens a
    /// fresh sled DB in a `tempdir` — callers may tweak settings via
    /// `state.read().await.db.set_setting(...)` before invoking the
    /// handler under test.
    fn test_state() -> (tempfile::TempDir, SharedState) {
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
        (dir, state)
    }

    /// Every `ModuleCategory` variant should have at least one module assigned.
    /// Catches registry drift where we introduce a category and forget to
    /// populate it (or vice versa).
    #[test]
    fn test_all_categories_represented() {
        let registry = module_registry_static();
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
        let registry = module_registry_static();

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
        let registry = module_registry_static();
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
        let registry = module_registry_static();
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
        for m in module_registry_static() {
            if let Some(route) = &m.ui_route {
                assert!(
                    route.starts_with('/'),
                    "ui_route of '{}' must start with '/' (got {route:?})",
                    m.name
                );
            }
        }
    }

    /// v2 invariant: the registry contains exactly 15 runtime-toggleable
    /// modules — the safe-to-flip surfaces (UI widgets, display-only
    /// integrations, experimental features). Expanding the list needs an
    /// explicit bump here so nobody silently flips `runtime_toggleable`
    /// for a security-sensitive module without updating the test.
    ///
    /// Modules that must **stay** non-toggleable: bookings, vehicles,
    /// rbac, sso, audit-export, multi-tenant, payments, stripe,
    /// invoices, webhooks, webhooks-v2, compliance, api-versioning,
    /// notifications, email, push, notification-center.
    #[test]
    fn test_runtime_toggleable_count() {
        let registry = module_registry_static();
        let toggleable: Vec<_> = registry
            .iter()
            .filter(|m| m.runtime_toggleable)
            .map(|m| m.name.clone())
            .collect();
        assert_eq!(
            toggleable.len(),
            15,
            "expected exactly 15 runtime-toggleable modules, got {}: {:?}",
            toggleable.len(),
            toggleable
        );

        // Spot-check: a security-sensitive module must NEVER be flagged
        // runtime_toggleable. If this fires, someone tried to unlock
        // the money path at runtime.
        for forbidden in [
            "bookings",
            "vehicles",
            "rbac",
            "sso",
            "audit-export",
            "payments",
            "stripe",
            "invoices",
            "webhooks",
            "webhooks-v2",
            "compliance",
            "multi-tenant",
            "notifications",
            "email",
            "push",
        ] {
            let m = registry
                .iter()
                .find(|m| m.name == forbidden)
                .unwrap_or_else(|| panic!("registry missing module '{forbidden}'"));
            assert!(
                !m.runtime_toggleable,
                "module '{forbidden}' must NOT be runtime_toggleable (security-sensitive)"
            );
        }
    }

    /// Every entry in [`MODULE_ROUTES`] must reference a real,
    /// runtime-toggleable module — catches typos + accidentally wiring
    /// the gate onto a non-toggleable module (where it would have no
    /// effect and only add latency).
    #[test]
    fn test_module_routes_reference_toggleable_modules() {
        let registry = module_registry_static();
        let toggleable: HashSet<&str> = registry
            .iter()
            .filter(|m| m.runtime_toggleable)
            .map(|m| m.name.as_str())
            .collect();
        for (module, prefixes) in MODULE_ROUTES {
            assert!(
                toggleable.contains(module),
                "MODULE_ROUTES references '{module}' which is not runtime_toggleable"
            );
            for prefix in *prefixes {
                assert!(
                    prefix.starts_with('/'),
                    "route prefix '{prefix}' for module '{module}' must start with '/'"
                );
            }
        }
    }

    /// Static view invariant: `runtime_enabled == enabled` because no
    /// override has been applied yet. `module_registry_static()` is the
    /// pure compile-time view.
    #[test]
    fn test_static_registry_runtime_equals_enabled() {
        for m in module_registry_static() {
            assert_eq!(
                m.runtime_enabled, m.enabled,
                "module '{}' static view must have runtime_enabled == enabled",
                m.name
            );
        }
    }

    /// Every config key must be snake_case (matches the admin settings store).
    #[test]
    fn test_config_keys_are_snake_case() {
        for m in module_registry_static() {
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

    /// Setting `module.{name}.runtime_enabled = "false"` on a
    /// runtime-toggleable, compile-time-enabled module must flip
    /// `runtime_enabled` to `false` in the async registry view.
    #[tokio::test]
    async fn test_setting_override_applies() {
        let (_dir, state) = test_state();

        // Pick any toggleable module that is compile-time enabled in
        // this test build. If the "full" feature set is on, map/graphql
        // are available; fall back to the first toggleable row that
        // happens to be enabled.
        let compile_enabled_toggleable: Vec<String> = module_registry_static()
            .into_iter()
            .filter(|m| m.runtime_toggleable && m.enabled)
            .map(|m| m.name)
            .collect();
        let Some(slug) = compile_enabled_toggleable.first().cloned() else {
            // No toggleable module is compile-time enabled in this test
            // feature set — skip. (Full test run always has modules
            // compiled.)
            return;
        };

        let state_guard = state.read().await;
        state_guard
            .db
            .set_setting(&runtime_enabled_setting_key(&slug), "false")
            .await
            .expect("set setting");

        let reg = module_registry(&state_guard.db).await;
        let m = reg.iter().find(|m| m.name == slug).expect("module present");
        assert!(
            !m.runtime_enabled,
            "module '{slug}' should be runtime-disabled after setting"
        );
        assert!(m.enabled, "compile-time enabled must stay true");
        assert!(m.runtime_toggleable);

        // Flip it back.
        state_guard
            .db
            .set_setting(&runtime_enabled_setting_key(&slug), "true")
            .await
            .expect("set setting");
        let reg = module_registry(&state_guard.db).await;
        let m = reg.iter().find(|m| m.name == slug).expect("module present");
        assert!(
            m.runtime_enabled,
            "module '{slug}' should be runtime-enabled again"
        );
    }

    /// Writing a runtime_enabled setting for a non-toggleable module
    /// must be silently ignored — `runtime_enabled` keeps equal to
    /// `enabled` regardless of the setting value.
    #[tokio::test]
    async fn test_non_toggleable_ignores_setting() {
        let (_dir, state) = test_state();

        // `bookings` is explicitly non-toggleable.
        let slug = "bookings";
        let state_guard = state.read().await;
        state_guard
            .db
            .set_setting(&runtime_enabled_setting_key(slug), "false")
            .await
            .expect("set setting");

        let reg = module_registry(&state_guard.db).await;
        let m = reg.iter().find(|m| m.name == slug).expect("bookings");
        assert!(!m.runtime_toggleable, "bookings must remain non-toggleable");
        assert_eq!(
            m.runtime_enabled, m.enabled,
            "non-toggleable module '{slug}' must ignore the setting"
        );
    }

    /// Handler shape: calling `list_modules` should return the full envelope
    /// with both the legacy Boolean map and the enriched array populated.
    /// The legacy Boolean map mirrors `runtime_enabled` (the effective
    /// state after override), not raw compile-time `enabled`.
    #[tokio::test]
    async fn test_list_modules_handler_shape() {
        let (_dir, state) = test_state();

        let Json(response) = list_modules(State(state)).await;

        // Registry has a healthy number of modules.
        assert!(
            response.module_info.len() >= 60,
            "expected at least 60 modules, got {}",
            response.module_info.len()
        );

        // Legacy map and enriched array carry the same entries. The
        // legacy map mirrors the effective `runtime_enabled` state so
        // clients that only read the flat map get the correct gated
        // view when a module is toggled off at runtime.
        assert_eq!(response.modules.len(), response.module_info.len());
        for m in &response.module_info {
            assert_eq!(
                response.modules.get(&m.name).copied(),
                Some(m.runtime_enabled),
                "modules[{}] should mirror module_info[].runtime_enabled",
                m.name
            );
        }

        // Version is the workspace version.
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    /// `get_module` returns 404 on unknown slugs.
    #[tokio::test]
    async fn test_get_module_unknown_returns_404() {
        let (_dir, state) = test_state();

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

    // ─────────────────────────────────────────────────────────────────
    // PATCH /api/v1/admin/modules/{name}
    // ─────────────────────────────────────────────────────────────────

    use parkhub_common::{User, UserPreferences, UserRole};

    /// Seed a user with the given role and return their `AuthUser`
    /// handle. Writes the user directly through `db.save_user` so the
    /// `check_admin` lookup succeeds.
    async fn seed_user(state: &SharedState, role: UserRole) -> AuthUser {
        let user_id = uuid::Uuid::new_v4();
        let user = User {
            id: user_id,
            username: format!("user-{}", &user_id.to_string()[..8]),
            email: format!("{user_id}@test.local"),
            password_hash: String::new(),
            name: "Test User".to_string(),
            picture: None,
            phone: None,
            role,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_login: None,
            preferences: UserPreferences::default(),
            is_active: true,
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            tenant_id: None,
            accessibility_needs: None,
            cost_center: None,
            department: None,
        };
        state
            .read()
            .await
            .db
            .save_user(&user)
            .await
            .expect("save user");
        AuthUser {
            user_id,
            api_key_id: None,
        }
    }

    /// PATCH flips a runtime-toggleable module's setting and returns the
    /// updated ModuleInfo with the new `runtime_enabled` state.
    #[tokio::test]
    async fn test_patch_admin_modules_toggles_setting() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        // Pick the first toggleable + compile-enabled module.
        let Some(slug) = module_registry_static()
            .into_iter()
            .find(|m| m.runtime_toggleable && m.enabled)
            .map(|m| m.name)
        else {
            return;
        };

        let (status, Json(response)) = patch_admin_module(
            State(state.clone()),
            axum::Extension(admin.clone()),
            Path(slug.clone()),
            Json(UpdateModuleRequest {
                runtime_enabled: false,
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let info = response.data.expect("updated module in response");
        assert_eq!(info.name, slug);
        assert!(!info.runtime_enabled, "should be disabled after PATCH");

        // Setting was persisted.
        let key = runtime_enabled_setting_key(&slug);
        let persisted = state
            .read()
            .await
            .db
            .get_setting(&key)
            .await
            .expect("get_setting")
            .expect("setting present");
        assert_eq!(persisted, "false");
    }

    /// PATCH on a non-toggleable module returns 409 CONFLICT and does
    /// NOT write the setting.
    #[tokio::test]
    async fn test_patch_returns_409_for_non_toggleable() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let (status, Json(response)) = patch_admin_module(
            State(state.clone()),
            axum::Extension(admin.clone()),
            Path("bookings".to_string()),
            Json(UpdateModuleRequest {
                runtime_enabled: false,
            }),
        )
        .await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert!(!response.success);

        // Setting must not have been written.
        let key = runtime_enabled_setting_key("bookings");
        assert!(
            state
                .read()
                .await
                .db
                .get_setting(&key)
                .await
                .expect("get_setting")
                .is_none(),
            "no setting must be persisted for a rejected PATCH"
        );
    }

    /// PATCH on an unknown slug returns 400 BAD_REQUEST.
    #[tokio::test]
    async fn test_patch_returns_400_for_unknown_module() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let (status, _) = patch_admin_module(
            State(state),
            axum::Extension(admin),
            Path("does-not-exist".to_string()),
            Json(UpdateModuleRequest {
                runtime_enabled: true,
            }),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// PATCH without admin role is rejected (403) and does not persist
    /// the setting.
    #[tokio::test]
    async fn test_patch_requires_admin() {
        let (_dir, state) = test_state();
        let user = seed_user(&state, UserRole::User).await;

        let Some(slug) = module_registry_static()
            .into_iter()
            .find(|m| m.runtime_toggleable && m.enabled)
            .map(|m| m.name)
        else {
            return;
        };

        let (status, _) = patch_admin_module(
            State(state.clone()),
            axum::Extension(user),
            Path(slug.clone()),
            Json(UpdateModuleRequest {
                runtime_enabled: false,
            }),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        let key = runtime_enabled_setting_key(&slug);
        assert!(
            state
                .read()
                .await
                .db
                .get_setting(&key)
                .await
                .expect("get_setting")
                .is_none(),
            "setting must not be written when caller is not admin"
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // module_gate middleware
    // ─────────────────────────────────────────────────────────────────

    /// `module_for_path` maps request paths to their owning module slug.
    #[test]
    fn test_module_for_path_matches_known_routes() {
        assert_eq!(module_for_path("/api/v1/lots/map"), Some("map"));
        assert_eq!(module_for_path("/api/v1/graphql"), Some("graphql"));
        assert_eq!(
            module_for_path("/api/v1/graphql/playground"),
            Some("graphql")
        );
        assert_eq!(module_for_path("/api/v1/docs"), Some("api-docs"));
        assert_eq!(
            module_for_path("/api/v1/docs/openapi.json"),
            Some("api-docs")
        );
        assert_eq!(
            module_for_path("/api/v1/announcements/active"),
            Some("announcements")
        );
        assert_eq!(module_for_path("/api/v1/user/favorites"), Some("favorites"));
        assert_eq!(
            module_for_path("/api/v1/user/favorites/abc-123"),
            Some("favorites")
        );

        // Non-owned paths pass through.
        assert_eq!(module_for_path("/api/v1/bookings"), None);
        assert_eq!(module_for_path("/health"), None);

        // Prefix must not match partials (`/api/v1/graphqlfoo` is not
        // `/api/v1/graphql/...`).
        assert_eq!(module_for_path("/api/v1/graphqlfoo"), None);
    }

    /// When a runtime-toggleable module is disabled via setting, the
    /// middleware short-circuits the request with 404 NOT_FOUND.
    #[tokio::test]
    async fn test_module_gate_blocks_when_disabled() {
        use axum::{Router, body::to_bytes, middleware as ax_mw, routing::get};
        use tower::ServiceExt;

        let (_dir, state) = test_state();

        // Disable `map` at runtime.
        state
            .read()
            .await
            .db
            .set_setting(&runtime_enabled_setting_key("map"), "false")
            .await
            .expect("set_setting");

        // Minimal router that only exists to test the gate.
        let app = Router::new()
            .route("/api/v1/lots/map", get(|| async { "ok" }))
            .route("/api/v1/lots/map/{id}", get(|| async { "ok" }))
            .route("/api/v1/bookings", get(|| async { "ok" }))
            .route_layer(ax_mw::from_fn_with_state(state.clone(), module_gate))
            .with_state(state.clone());

        // Gated path returns 404.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/lots/map")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body_bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["error"]["code"], "MODULE_DISABLED");

        // Sub-path under the same module also 404s.
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/lots/map/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve");
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        // Non-gated path passes through.
        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/bookings")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve");
        assert_eq!(res.status(), StatusCode::OK);
    }

    /// When the module is enabled (default / no setting), the middleware
    /// is transparent.
    #[tokio::test]
    async fn test_module_gate_passes_when_enabled() {
        use axum::{Router, middleware as ax_mw, routing::get};
        use tower::ServiceExt;

        let (_dir, state) = test_state();

        let app = Router::new()
            .route("/api/v1/lots/map", get(|| async { "ok" }))
            .route_layer(ax_mw::from_fn_with_state(state.clone(), module_gate))
            .with_state(state);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/lots/map")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve");
        assert_eq!(res.status(), StatusCode::OK);
    }
}
