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
//!
//! ## Module layout
//!
//! This file is the public facade — types, shared helpers, and the
//! async [`module_registry`] registry accessor. The declarative
//! registry table lives in [`registry`], HTTP handlers in [`handlers`],
//! the runtime-gating middleware in [`gate`], and per-module JSON
//! Schema literals in [`schemas`]. Tests live in [`tests`].

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use super::{AuthUser, SharedState, check_admin};

mod gate;
mod handlers;
mod registry;
mod schemas;
#[cfg(test)]
mod tests;

// ─── Re-exports — keep the external API surface stable after the split. ───

#[allow(unused_imports)]
pub use gate::{MODULE_ROUTES, module_for_path, module_gate};
pub use handlers::{
    get_module, get_module_config, list_modules, patch_admin_module, patch_module_config,
};
pub use registry::module_registry_static;

// The `#[utoipa::path(...)]` macro generates companion `__path_<fn>` types in
// the same module as the decorated function. Re-export them so the OpenApi
// derive in `crate::openapi` (which references the handlers as
// `crate::api::modules::<fn>`) still resolves the generated path types after
// the split. These names are internal to utoipa and not a user-facing API.
#[doc(hidden)]
pub use handlers::{
    __path_get_module, __path_get_module_config, __path_list_modules, __path_patch_admin_module,
    __path_patch_module_config,
};

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

// ═════════════════════════════════════════════════════════════════════════════
// Shared helpers
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

/// Compile-time validate a raw schema literal → runtime [`ConfigSchema`].
///
/// Returns `Err(String)` when the literal is not a JSON object. Handler
/// code turns this into a 500 with a clear log line — the
/// `test_config_schema_strings_are_valid_json` test rules out shipping
/// broken literals in the first place.
pub(super) fn parse_schema(literal: &str) -> Result<(serde_json::Value, ConfigSchema), String> {
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
pub(super) async fn load_module_values(
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

/// Validate an instance against a JSON Schema document, returning a
/// list of human-readable error messages on failure.
///
/// The errors carry enough context to surface in a form UI (`/foo:
/// message`) without leaking internal state.
pub(super) fn validate_instance(
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
        .map(|e| format!("{} at `{}`", e, e.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
