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
//!
//! ## v3 — per-module JSON Schema config editor
//!
//! Each row can optionally declare a JSON Schema (draft 2020-12) string
//! that describes the module's admin-settings payload. Modules that
//! declare one get two admin-only endpoints:
//!
//! - `GET /api/v1/admin/modules/{name}/config` — returns the schema and
//!   the currently stored values (looked up via admin-settings keys
//!   `module.{name}.config.{field}`).
//! - `PATCH /api/v1/admin/modules/{name}/config` — validates the request
//!   body against the schema and, on success, writes each field back to
//!   the settings store. Every successful write emits an
//!   [`AuditEventType::ConfigChanged`] entry with the list of changed
//!   keys.
//!
//! Modules without a `config_schema` entry return `400 BAD_REQUEST` on
//! both endpoints — there is no "empty schema" fallback to avoid
//! accidentally shipping an editor for a module whose settings story
//! hasn't been codified yet.

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

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
// Module config schemas (T-1720 v3)
// ═════════════════════════════════════════════════════════════════════════════
//
// One const string per module that exposes a config editor. We keep the
// schemas as literal JSON so (a) they are trivially reviewable in diffs,
// (b) they cost zero runtime parsing effort beyond the one-time registry
// materialisation, and (c) the set of declared modules is obvious from
// `rg MOD_.*_SCHEMA`.
//
// Every schema is draft 2020-12, `additionalProperties: false`, and only
// enumerates fields that the Settings store actually consumes today. We
// deliberately do *not* invent aspirational fields — better to extend
// the schema when a setting is added than to ship a UI for a
// non-existent key.

/// `mod-themes` — tenant default theme + per-user override flag.
const MOD_THEMES_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Themes settings",
  "description": "Default theme for the tenant plus a flag that gates per-user theme override.",
  "properties": {
    "default_theme": {
      "type": "string",
      "enum": ["light", "dark", "classic"],
      "description": "Theme used when a user has not picked one."
    },
    "allow_user_override": {
      "type": "boolean",
      "description": "When true, individual users can pick their own theme."
    }
  },
  "required": ["default_theme", "allow_user_override"],
  "additionalProperties": false
}"#;

/// `mod-announcements` — admin banner policy.
const MOD_ANNOUNCEMENTS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Announcements settings",
  "description": "Controls the admin-published banner system.",
  "properties": {
    "max_announcements": {
      "type": "integer",
      "minimum": 1,
      "maximum": 50,
      "description": "Maximum number of simultaneously active announcements."
    },
    "default_ttl_days": {
      "type": "integer",
      "minimum": 1,
      "maximum": 365,
      "description": "Default days until an announcement auto-expires."
    },
    "show_on_login": {
      "type": "boolean",
      "description": "Display the active announcement list on the login page."
    }
  },
  "required": ["max_announcements", "default_ttl_days", "show_on_login"],
  "additionalProperties": false
}"#;

/// `mod-notifications` — delivery channels and quiet hours.
///
/// `quiet_hours_start` / `quiet_hours_end` use an `HH:MM` 24-hour pattern
/// rather than the full RFC 3339 `time` format because we only ever
/// consume minutes-of-day; seconds + timezone would be silently
/// discarded otherwise.
const MOD_NOTIFICATIONS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Notifications settings",
  "description": "Master switches per channel plus nightly quiet-hours window.",
  "properties": {
    "push_enabled": {
      "type": "boolean",
      "description": "Send Web Push notifications to subscribed clients."
    },
    "email_enabled": {
      "type": "boolean",
      "description": "Send transactional email notifications."
    },
    "quiet_hours_start": {
      "type": "string",
      "pattern": "^([01][0-9]|2[0-3]):[0-5][0-9]$",
      "description": "Start of the nightly quiet window (HH:MM, 24h)."
    },
    "quiet_hours_end": {
      "type": "string",
      "pattern": "^([01][0-9]|2[0-3]):[0-5][0-9]$",
      "description": "End of the nightly quiet window (HH:MM, 24h)."
    }
  },
  "required": ["push_enabled", "email_enabled", "quiet_hours_start", "quiet_hours_end"],
  "additionalProperties": false
}"#;

/// `mod-email-templates` — outbound envelope identity.
const MOD_EMAIL_TEMPLATES_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Email template settings",
  "description": "Envelope identity applied to every transactional email.",
  "properties": {
    "from_address": {
      "type": "string",
      "format": "email",
      "description": "`From:` address on outbound email."
    },
    "from_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 128,
      "description": "Human-readable display name on outbound email."
    },
    "reply_to": {
      "type": "string",
      "format": "email",
      "description": "`Reply-To:` address, typically a monitored inbox."
    }
  },
  "required": ["from_address", "from_name", "reply_to"],
  "additionalProperties": false
}"#;

/// `mod-widgets` — embeddable dashboard widget cap.
const MOD_WIDGETS_SCHEMA: &str = r#"{
  "type": "object",
  "title": "Widgets settings",
  "description": "Limits for the embeddable widgets subsystem.",
  "properties": {
    "max_widgets_per_dashboard": {
      "type": "integer",
      "minimum": 1,
      "maximum": 20,
      "description": "Maximum number of widgets that can be pinned to one dashboard."
    }
  },
  "required": ["max_widgets_per_dashboard"],
  "additionalProperties": false
}"#;

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
    /// Optional JSON Schema (draft 2020-12) literal describing the
    /// module's admin config payload.
    ///
    /// When `Some(_)`, the module gains a pair of admin-only endpoints
    /// under `/api/v1/admin/modules/{name}/config`. `None` — the default
    /// for every module — means the module has no generic config editor
    /// and both endpoints return `400 BAD_REQUEST`.
    ///
    /// The literal must parse as a valid JSON object; [`module_registry_static`]
    /// validates the shape on startup / in tests (see
    /// `test_config_schema_strings_are_valid_json`).
    config_schema: Option<&'static str>,
}

/// Wrapper that transparently serialises a JSON Schema document (draft
/// 2020-12) into the API payload while still giving utoipa a handle on
/// the field for schema generation.
///
/// The inner `schema` is a `serde_json::Value` because JSON Schema
/// documents are themselves unbounded in shape — encoding it as an
/// arbitrary `Object` lets us round-trip any valid schema without
/// forcing a second-order type. `#[serde(flatten)]` pulls the schema's
/// top-level keys (`type`, `properties`, `required`, …) onto the
/// response envelope so clients see the raw schema, not a
/// `{schema: {...}}` wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConfigSchema {
    /// JSON Schema document (draft 2020-12) describing the module's
    /// settings. Transparent — fields are flattened into the parent
    /// response.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub schema: serde_json::Value,
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
    /// Parsed JSON Schema for the module's admin config editor.
    ///
    /// `Some(_)` when the module declared a literal `config_schema` in
    /// its [`ModuleDef`] row; `None` otherwise. Present in
    /// `/api/v1/modules` so the admin dashboard can decide whether to
    /// render an "Edit config" affordance without a second round trip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<ConfigSchema>,
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
///
/// A malformed `config_schema` string (non-JSON) is treated as "no
/// schema declared" for this row: we emit a `tracing::error!` and set
/// `config_schema = None` rather than panicking, so one typo doesn't
/// take the entire modules endpoint down. The
/// `test_config_schema_strings_are_valid_json` test guarantees we never
/// actually ship a broken literal.
fn materialize(def: &ModuleDef) -> ModuleInfo {
    let config_schema =
        def.config_schema
            .and_then(|src| match serde_json::from_str::<serde_json::Value>(src) {
                Ok(schema) => Some(ConfigSchema { schema }),
                Err(e) => {
                    tracing::error!(
                        module = def.name,
                        error = %e,
                        "module config_schema literal is not valid JSON; dropping"
                    );
                    None
                }
            });

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
        config_schema,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: Some(MOD_NOTIFICATIONS_SCHEMA),
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
            config_schema: None,
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
            config_schema: Some(MOD_ANNOUNCEMENTS_SCHEMA),
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
            config_schema: None,
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
            config_schema: Some(MOD_EMAIL_TEMPLATES_SCHEMA),
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: Some(MOD_THEMES_SCHEMA),
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: Some(MOD_WIDGETS_SCHEMA),
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
            config_schema: None,
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
// Per-module JSON Schema config editor (T-1720 v3)
// ═════════════════════════════════════════════════════════════════════════════

/// Response + request body for the config editor. Carries both the
/// schema (so the UI can render a form) and the current persisted values
/// (so the UI can pre-fill fields). The PATCH request shape reuses the
/// same struct: send `{"values": {...}}` and leave `schema` empty /
/// ignored.
///
/// Using one struct for both directions keeps the client contract
/// symmetric and means a `PATCH` response can be fed straight back into
/// the editor without renormalisation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleConfig {
    /// Parsed schema for this module, echoed back so clients don't need
    /// to re-fetch it after a PATCH. On PATCH requests this field is
    /// ignored — the server is the source of truth for schemas.
    pub schema: ConfigSchema,
    /// Currently persisted values, one entry per schema `properties` key
    /// that has a stored value. Keys with no stored value are omitted;
    /// the UI falls back to schema defaults (or an empty form field).
    pub values: HashMap<String, serde_json::Value>,
}

/// PATCH request body — only `values`, no schema.
///
/// Kept as a separate struct (instead of reusing [`ModuleConfig`]) so
/// the OpenAPI request-body schema doesn't accept a (server-owned)
/// `schema` field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateModuleConfigRequest {
    /// New values keyed by schema property name. Each value must pass
    /// the schema validator for its property.
    pub values: HashMap<String, serde_json::Value>,
}

/// Build the admin-settings key that stores one field of a module's
/// runtime config.
///
/// Shape: `module.{slug}.config.{field}`. Each field is stored as a
/// JSON-encoded string so we can round-trip any schema type (boolean,
/// integer, string, object, …) through the string-typed settings
/// store.
#[must_use]
pub fn config_setting_key(module_name: &str, field: &str) -> String {
    format!("module.{module_name}.config.{field}")
}

/// Compile-time validate a raw schema literal → runtime [`ConfigSchema`].
///
/// Returns `Err(String)` when the literal is not a JSON object. Handler
/// code turns this into a 500 with a clear log line — the
/// `test_config_schema_strings_are_valid_json` test rules out shipping
/// broken literals in the first place.
fn parse_schema(literal: &str) -> Result<(serde_json::Value, ConfigSchema), String> {
    let value: serde_json::Value = serde_json::from_str(literal)
        .map_err(|e| format!("schema literal is not valid JSON: {e}"))?;
    Ok((value.clone(), ConfigSchema { schema: value }))
}

/// Read every value currently persisted for a module, keyed off the
/// schema's `properties` object.
///
/// Keys that have no stored value are omitted from the returned map.
/// Values that failed to JSON-decode (e.g. a hand-edited row) are also
/// skipped with an error log rather than poisoning the entire response.
async fn load_module_values(
    db: &crate::db::Database,
    module_name: &str,
    schema_value: &serde_json::Value,
) -> HashMap<String, serde_json::Value> {
    let Some(properties) = schema_value.get("properties").and_then(|p| p.as_object()) else {
        return HashMap::new();
    };

    let mut out = HashMap::new();
    for field in properties.keys() {
        let key = config_setting_key(module_name, field);
        match db.get_setting(&key).await {
            Ok(Some(raw)) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(value) => {
                    out.insert(field.clone(), value);
                }
                Err(e) => {
                    tracing::error!(
                        module = module_name,
                        field = %field,
                        error = %e,
                        "stored module config value is not valid JSON; skipping"
                    );
                }
            },
            Ok(None) => { /* unset — fine */ }
            Err(e) => {
                tracing::error!(
                    module = module_name,
                    field = %field,
                    error = %e,
                    "settings store read failed"
                );
            }
        }
    }
    out
}

/// Resolve a module slug to its parsed schema, or return the appropriate
/// HTTP error. Centralises the 404 / 400 distinction shared by the
/// `GET` and `PATCH` handlers.
///
/// - `Err(NOT_FOUND)` — slug is not in the registry at all.
/// - `Err(BAD_REQUEST)` — slug exists but has no `config_schema`
///   declared (no generic editor available for this module).
fn resolve_schema(name: &str) -> Result<(serde_json::Value, ConfigSchema), StatusCode> {
    let Some(def) = registry_defs().into_iter().find(|d| d.name == name) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let Some(literal) = def.config_schema else {
        return Err(StatusCode::BAD_REQUEST);
    };
    parse_schema(literal).map_err(|e| {
        tracing::error!(module = name, error = %e, "module schema literal is broken");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// `GET /api/v1/admin/modules/{name}/config` — schema + current values.
///
/// Admin-only. Returns:
/// - `200 OK` with `{schema, values}` when the module declares a schema.
/// - `403 FORBIDDEN` when the caller is not an admin (defense-in-depth;
///   the admin_middleware layer is the primary guard).
/// - `404 NOT_FOUND` when the slug is not in the registry.
/// - `400 BAD_REQUEST` when the module exists but has no schema.
#[utoipa::path(
    get,
    path = "/api/v1/admin/modules/{name}/config",
    tag = "Admin",
    summary = "Read a module's config schema + current values",
    description = "Admin-only. Returns the JSON Schema (draft 2020-12) describing the module's admin config payload plus the currently persisted values. Returns 404 when the slug is unknown and 400 when the module has no schema declared.",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Module slug (kebab-case)")),
    responses(
        (status = 200, description = "Config schema + values", body = ModuleConfig),
        (status = 400, description = "Module has no config schema"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Unknown module slug")
    )
)]
pub async fn get_module_config(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Path(name): Path<String>,
) -> Result<Json<ModuleConfig>, (StatusCode, Json<ApiResponse<()>>)> {
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return Err((status, Json(ApiResponse::error("FORBIDDEN", msg))));
    }

    let (schema_value, schema) = resolve_schema(&name).map_err(|status| match status {
        StatusCode::NOT_FOUND => (
            status,
            Json(ApiResponse::error(
                "UNKNOWN_MODULE",
                format!("Unknown module slug: {name}"),
            )),
        ),
        StatusCode::BAD_REQUEST => (
            status,
            Json(ApiResponse::error(
                "NO_CONFIG_SCHEMA",
                format!("Module '{name}' has no config schema"),
            )),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to resolve module schema",
            )),
        ),
    })?;

    let values = load_module_values(&state_guard.db, &name, &schema_value).await;
    drop(state_guard);

    Ok(Json(ModuleConfig { schema, values }))
}

/// Validate an instance against a JSON Schema document, returning a
/// list of human-readable error messages on failure.
///
/// The errors carry enough context to surface in a form UI (`/foo:
/// message`) without leaking internal state.
fn validate_instance(
    schema_value: &serde_json::Value,
    instance: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let validator = match jsonschema::draft202012::new(schema_value) {
        Ok(v) => v,
        Err(e) => {
            // If the literal is broken we've already returned 500 from
            // `resolve_schema`; reaching here is a programmer error.
            return Err(vec![format!("schema compile error: {e}")]);
        }
    };
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{} at `{}`", e, e.instance_path))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// `PATCH /api/v1/admin/modules/{name}/config` — write new values.
///
/// Admin-only. Body is `{"values": {...}}`. The full
/// `values` object is validated against the module's schema in one shot
/// (so `required` / `additionalProperties: false` / cross-field
/// constraints all apply). On success every field is JSON-encoded and
/// persisted to `module.{name}.config.{field}`.
///
/// Emits an `AuditEventType::ConfigChanged` entry carrying the caller's
/// user_id, the module slug, and the list of changed keys. The entry is
/// written even if some keys already had the new value — the caller's
/// *intent* is what we audit.
///
/// Error map:
/// - `400 BAD_REQUEST` — module has no schema.
/// - `403 FORBIDDEN` — caller is not an admin.
/// - `404 NOT_FOUND` — unknown slug.
/// - `422 UNPROCESSABLE_ENTITY` — body fails schema validation. The
///   response carries a `details.errors: [...]` array with one string
///   per violation for form-level display.
#[utoipa::path(
    patch,
    path = "/api/v1/admin/modules/{name}/config",
    tag = "Admin",
    summary = "Update a module's config values",
    description = "Admin-only. Validates the `values` body against the module's JSON Schema. On success writes each field to `module.{name}.config.{field}` in the settings store and emits a `ConfigChanged` audit entry. Returns 422 with a per-violation error list when validation fails.",
    security(("bearer_auth" = [])),
    params(("name" = String, Path, description = "Module slug (kebab-case)")),
    request_body = UpdateModuleConfigRequest,
    responses(
        (status = 200, description = "Updated config", body = ModuleConfig),
        (status = 400, description = "Module has no config schema"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Unknown module slug"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn patch_module_config(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Path(name): Path<String>,
    Json(body): Json<UpdateModuleConfigRequest>,
) -> (StatusCode, Json<ApiResponse<ModuleConfig>>) {
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let (schema_value, schema) = match resolve_schema(&name) {
        Ok(pair) => pair,
        Err(StatusCode::NOT_FOUND) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "UNKNOWN_MODULE",
                    format!("Unknown module slug: {name}"),
                )),
            );
        }
        Err(StatusCode::BAD_REQUEST) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "NO_CONFIG_SCHEMA",
                    format!("Module '{name}' has no config schema"),
                )),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to resolve module schema",
                )),
            );
        }
    };

    // Validate the full `values` object (not each field in isolation) so
    // `required`, `additionalProperties: false`, and any cross-field
    // schema constraints all apply together.
    let instance = serde_json::to_value(&body.values).unwrap_or(serde_json::Value::Null);
    if let Err(errors) = validate_instance(&schema_value, &instance) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse {
                success: false,
                data: None,
                error: Some(parkhub_common::ApiError {
                    code: "VALIDATION_FAILED".to_string(),
                    message: "Request body failed schema validation".to_string(),
                    details: Some(serde_json::json!({ "errors": errors })),
                }),
                meta: None,
            }),
        );
    }

    // Persist — each field is JSON-encoded so we can round-trip any
    // schema type through the string-typed settings store.
    let mut keys_changed: Vec<String> = Vec::with_capacity(body.values.len());
    for (field, value) in &body.values {
        let key = config_setting_key(&name, field);
        let encoded = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    module = %name,
                    field = %field,
                    error = %e,
                    "failed to encode module config value"
                );
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Failed to encode config value",
                    )),
                );
            }
        };
        if let Err(e) = state_guard.db.set_setting(&key, &encoded).await {
            tracing::error!(
                module = %name,
                field = %field,
                error = %e,
                "failed to persist module config value"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to save module config",
                )),
            );
        }
        keys_changed.push(field.clone());
    }
    // Stable order for the audit entry + tests.
    keys_changed.sort();

    // Audit log — ConfigChanged with the list of touched keys.
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
                "module": name,
                "keys_changed": keys_changed,
            }))
            .log();
        entry.persist(&state_guard.db).await;
    }

    // Re-read persisted values so the response reflects the new state
    // exactly as a subsequent GET would.
    let values = load_module_values(&state_guard.db, &name, &schema_value).await;
    drop(state_guard);

    (
        StatusCode::OK,
        Json(ApiResponse::success(ModuleConfig { schema, values })),
    )
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

    // ─────────────────────────────────────────────────────────────────
    // T-1720 v3 — per-module JSON Schema config editor
    // ─────────────────────────────────────────────────────────────────

    /// Every module that declares a `config_schema` literal must parse
    /// as valid JSON *and* compile as a draft 2020-12 schema. A typo in
    /// a schema literal would otherwise only surface at runtime when an
    /// admin hits the endpoint.
    #[test]
    fn test_config_schema_strings_are_valid_json() {
        for def in registry_defs() {
            let Some(literal) = def.config_schema else {
                continue;
            };
            let value: serde_json::Value = serde_json::from_str(literal).unwrap_or_else(|e| {
                panic!("module '{}' has invalid config_schema JSON: {e}", def.name)
            });
            jsonschema::draft202012::new(&value).unwrap_or_else(|e| {
                panic!(
                    "module '{}' config_schema does not compile as draft 2020-12: {e}",
                    def.name
                )
            });
        }
    }

    /// The five modules that ship a schema in v3: themes,
    /// announcements, notifications, email-templates, widgets. Other
    /// modules intentionally keep `config_schema: None`.
    #[test]
    fn test_expected_modules_have_schemas() {
        let with_schema: std::collections::HashSet<String> = registry_defs()
            .into_iter()
            .filter(|d| d.config_schema.is_some())
            .map(|d| d.name.to_string())
            .collect();
        let expected: std::collections::HashSet<String> = [
            "themes",
            "announcements",
            "notifications",
            "email-templates",
            "widgets",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(
            with_schema, expected,
            "expected exactly the v3 5-module set to ship a schema"
        );
    }

    /// GET returns 200 OK with the themes schema + empty values on a
    /// fresh DB. Once a value is persisted, the GET reflects it.
    #[tokio::test]
    async fn test_get_config_returns_schema_and_values_for_themes() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        // Empty DB → schema present, values empty.
        let result = get_module_config(
            State(state.clone()),
            axum::Extension(admin.clone()),
            Path("themes".to_string()),
        )
        .await;
        let Json(cfg) = result.expect("200 OK");
        assert!(cfg.schema.schema.is_object(), "schema must be an object");
        assert!(cfg.values.is_empty(), "fresh DB has no values");

        // Persist a value directly and confirm GET surfaces it.
        state
            .read()
            .await
            .db
            .set_setting(&config_setting_key("themes", "default_theme"), "\"dark\"")
            .await
            .expect("set_setting");
        let result = get_module_config(
            State(state.clone()),
            axum::Extension(admin),
            Path("themes".to_string()),
        )
        .await;
        let Json(cfg) = result.expect("200 OK");
        assert_eq!(
            cfg.values.get("default_theme"),
            Some(&serde_json::Value::String("dark".to_string()))
        );
    }

    /// GET with an unknown slug returns 404 NOT_FOUND.
    #[tokio::test]
    async fn test_get_config_404_for_unknown_module() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let result = get_module_config(
            State(state),
            axum::Extension(admin),
            Path("does-not-exist".to_string()),
        )
        .await;
        let err = result.expect_err("404");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    /// GET on a module that has no `config_schema` declared returns 400
    /// BAD_REQUEST. `bookings` is picked because it ships with the
    /// registry but has no schema in v3.
    #[tokio::test]
    async fn test_get_config_400_for_module_without_schema() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let result = get_module_config(
            State(state),
            axum::Extension(admin),
            Path("bookings".to_string()),
        )
        .await;
        let err = result.expect_err("400");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    /// Non-admin callers get 403 FORBIDDEN. The admin_middleware layer
    /// is the primary guard in production, but the handler re-checks
    /// defense-in-depth.
    #[tokio::test]
    async fn test_get_config_403_for_non_admin() {
        let (_dir, state) = test_state();
        let user = seed_user(&state, UserRole::User).await;

        let result = get_module_config(
            State(state),
            axum::Extension(user),
            Path("themes".to_string()),
        )
        .await;
        let err = result.expect_err("403");
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    /// PATCH rejects values whose **type** does not match the schema
    /// (integer where a string is required).
    #[tokio::test]
    async fn test_patch_config_validates_types() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let mut values = HashMap::new();
        values.insert(
            "default_theme".to_string(),
            serde_json::Value::Number(42.into()),
        );
        values.insert("allow_user_override".to_string(), serde_json::json!(true));

        let (status, Json(response)) = patch_module_config(
            State(state.clone()),
            axum::Extension(admin),
            Path("themes".to_string()),
            Json(UpdateModuleConfigRequest { values }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let err = response.error.expect("error payload");
        assert_eq!(err.code, "VALIDATION_FAILED");
        let errors = err
            .details
            .as_ref()
            .and_then(|d| d.get("errors"))
            .and_then(|e| e.as_array())
            .expect("details.errors");
        assert!(!errors.is_empty(), "at least one violation expected");

        // Nothing should have been persisted.
        let raw = state
            .read()
            .await
            .db
            .get_setting(&config_setting_key("themes", "default_theme"))
            .await
            .expect("get_setting");
        assert!(raw.is_none(), "rejected PATCH must not persist anything");
    }

    /// PATCH rejects values outside the schema's `enum` constraint.
    #[tokio::test]
    async fn test_patch_config_validates_enum() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let mut values = HashMap::new();
        values.insert(
            "default_theme".to_string(),
            serde_json::Value::String("neon".to_string()),
        );
        values.insert("allow_user_override".to_string(), serde_json::json!(true));

        let (status, Json(response)) = patch_module_config(
            State(state),
            axum::Extension(admin),
            Path("themes".to_string()),
            Json(UpdateModuleConfigRequest { values }),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.error.expect("error").code, "VALIDATION_FAILED");
    }

    /// PATCH persists values and a subsequent GET returns them. Confirms
    /// each value is JSON-encoded in the settings store (so we can
    /// round-trip any schema type through the string-typed store).
    #[tokio::test]
    async fn test_patch_config_persists_values() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        let mut values = HashMap::new();
        values.insert(
            "default_theme".to_string(),
            serde_json::Value::String("dark".to_string()),
        );
        values.insert("allow_user_override".to_string(), serde_json::json!(false));

        let (status, Json(response)) = patch_module_config(
            State(state.clone()),
            axum::Extension(admin.clone()),
            Path("themes".to_string()),
            Json(UpdateModuleConfigRequest {
                values: values.clone(),
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let cfg = response.data.expect("config payload");
        assert_eq!(
            cfg.values.get("default_theme"),
            Some(&serde_json::Value::String("dark".to_string()))
        );
        assert_eq!(
            cfg.values.get("allow_user_override"),
            Some(&serde_json::json!(false))
        );

        // Raw setting is JSON-encoded.
        let raw = state
            .read()
            .await
            .db
            .get_setting(&config_setting_key("themes", "default_theme"))
            .await
            .expect("get_setting")
            .expect("value present");
        assert_eq!(raw, "\"dark\"");

        // Round-trip via GET.
        let result = get_module_config(
            State(state),
            axum::Extension(admin),
            Path("themes".to_string()),
        )
        .await;
        let Json(cfg) = result.expect("200 OK");
        assert_eq!(
            cfg.values.get("default_theme"),
            Some(&serde_json::Value::String("dark".to_string()))
        );
    }

    /// Successful PATCH emits a `ConfigChanged` audit entry naming the
    /// module and the changed keys.
    #[tokio::test]
    async fn test_patch_config_audit_log_entry_exists() {
        let (_dir, state) = test_state();
        let admin = seed_user(&state, UserRole::Admin).await;

        // `test_state()` uses `ServerConfig::default()`; make sure audit
        // logging is on for this test — otherwise the handler correctly
        // skips the persist step.
        {
            let mut guard = state.write().await;
            guard.config.audit_logging_enabled = true;
        }

        let mut values = HashMap::new();
        values.insert(
            "default_theme".to_string(),
            serde_json::Value::String("classic".to_string()),
        );
        values.insert("allow_user_override".to_string(), serde_json::json!(true));

        let (status, _) = patch_module_config(
            State(state.clone()),
            axum::Extension(admin),
            Path("themes".to_string()),
            Json(UpdateModuleConfigRequest { values }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let entries = state
            .read()
            .await
            .db
            .list_audit_log(100)
            .await
            .expect("list_audit_log");
        // DB-persisted audit entries encode `event_type` via `Debug`
        // formatting of `AuditEventType` (see `audit::AuditEntry::persist`)
        // and `details` as the `ToString` of the JSON value — we parse it
        // back here to inspect the structured payload.
        let cfg_entries: Vec<_> = entries
            .iter()
            .filter(|e| e.event_type == "ConfigChanged")
            .filter(|e| e.target_type.as_deref() == Some("module"))
            .filter(|e| e.target_id.as_deref() == Some("themes"))
            .collect();
        assert_eq!(
            cfg_entries.len(),
            1,
            "exactly one ConfigChanged entry for 'themes' expected"
        );
        let details_raw = cfg_entries[0].details.as_ref().expect("details present");
        let details: serde_json::Value =
            serde_json::from_str(details_raw).expect("details is JSON");
        assert_eq!(
            details.get("module").and_then(|v| v.as_str()),
            Some("themes")
        );
        let keys = details
            .get("keys_changed")
            .and_then(|v| v.as_array())
            .expect("keys_changed array");
        let mut key_strs: Vec<String> = keys
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        key_strs.sort();
        assert_eq!(key_strs, vec!["allow_user_override", "default_theme"]);
    }
}
