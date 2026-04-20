//! HTTP handlers for the module registry + config editor endpoints.
//!
//! Serves the public `/api/v1/modules` + `/api/v1/modules/{name}`
//! registry views, plus the admin-only runtime-toggle and per-module
//! JSON Schema config editor under `/api/v1/admin/modules/{name}`.

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use parkhub_common::ApiResponse;
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::audit::{AuditEntry, AuditEventType};

use super::registry::{module_registry_static, registry_defs};
use super::{
    AuthUser, ConfigSchema, ListModulesResponse, ModuleConfig, ModuleInfo, SharedState,
    UpdateModuleConfigRequest, UpdateModuleRequest, canonical_module_slug, check_admin,
    config_setting_key, load_module_values, module_registry, parse_schema,
    runtime_enabled_setting_key, validate_instance,
};

#[derive(Debug, Clone, Serialize, ToSchema)]
struct PublicApiErrorSchema {
    code: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct PublicResponseMetaSchema {
    page: Option<i32>,
    per_page: Option<i32>,
    total: Option<i32>,
    total_pages: Option<i32>,
}

macro_rules! define_public_response_schema {
    ($name:ident, $payload:ty) => {
        #[derive(Debug, Clone, Serialize, ToSchema)]
        struct $name {
            success: bool,
            data: Option<$payload>,
            error: Option<PublicApiErrorSchema>,
            meta: Option<PublicResponseMetaSchema>,
        }
    };
}

define_public_response_schema!(ListModulesResponseSchema, ListModulesResponse);
define_public_response_schema!(ModuleInfoResponseSchema, ModuleInfo);
define_public_response_schema!(ModuleConfigResponseSchema, ModuleConfig);

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
    responses((status = 200, description = "Module registry", body = ListModulesResponseSchema))
)]
pub async fn list_modules(
    State(state): State<SharedState>,
) -> Json<ApiResponse<ListModulesResponse>> {
    let state_guard = state.read().await;
    let info = module_registry(&state_guard.db).await;
    drop(state_guard);
    let modules: HashMap<String, bool> = info
        .iter()
        .map(|m| (m.name.clone(), m.runtime_enabled))
        .collect();
    Json(ApiResponse::success(ListModulesResponse {
        modules,
        module_info: info,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
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
        (status = 200, description = "Module metadata", body = ModuleInfoResponseSchema),
        (status = 404, description = "Unknown module slug", body = ModuleInfoResponseSchema)
    )
)]
pub async fn get_module(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> (StatusCode, Json<ApiResponse<ModuleInfo>>) {
    let canonical_name = canonical_module_slug(&name).to_string();
    let state_guard = state.read().await;
    let info = module_registry(&state_guard.db).await;
    drop(state_guard);
    match info.into_iter().find(|m| m.name == canonical_name) {
        Some(module) => (StatusCode::OK, Json(ApiResponse::success(module))),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "UNKNOWN_MODULE",
                format!("Unknown module slug: {name}"),
            )),
        ),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// Runtime toggle — PATCH /api/v1/admin/modules/{name}
// ═════════════════════════════════════════════════════════════════════════════

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
        (status = 200, description = "Updated module state", body = ModuleInfoResponseSchema),
        (status = 400, description = "Unknown module slug", body = ModuleInfoResponseSchema),
        (status = 403, description = "Admin access required", body = ModuleInfoResponseSchema),
        (status = 409, description = "Module is not runtime_toggleable", body = ModuleInfoResponseSchema)
    )
)]
pub async fn patch_admin_module(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Path(name): Path<String>,
    Json(body): Json<UpdateModuleRequest>,
) -> (StatusCode, Json<ApiResponse<ModuleInfo>>) {
    let canonical_name = canonical_module_slug(&name).to_string();
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
        .find(|m| m.name == canonical_name)
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

    let key = runtime_enabled_setting_key(&canonical_name);
    let value = if body.runtime_enabled {
        "true"
    } else {
        "false"
    };
    if let Err(e) = state_guard.db.set_setting(&key, value).await {
        tracing::error!(module = %canonical_name, error = %e, "Failed to persist module runtime toggle");
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
            .resource("module", &canonical_name)
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
    let Some(updated) = info.into_iter().find(|m| m.name == canonical_name) else {
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

/// Resolve a module slug to its parsed schema, or return the appropriate
/// HTTP error. Centralises the 404 / 400 distinction shared by the
/// `GET` and `PATCH` handlers.
///
/// - `Err(NOT_FOUND)` — slug is not in the registry at all.
/// - `Err(BAD_REQUEST)` — slug exists but has no `config_schema`
///   declared (no generic editor available for this module).
pub(super) fn resolve_schema(name: &str) -> Result<(serde_json::Value, ConfigSchema), StatusCode> {
    let canonical_name = canonical_module_slug(name);
    let Some(def) = registry_defs()
        .into_iter()
        .find(|d| d.name == canonical_name)
    else {
        return Err(StatusCode::NOT_FOUND);
    };
    let Some(literal) = def.config_schema else {
        return Err(StatusCode::BAD_REQUEST);
    };
    parse_schema(literal).map_err(|e| {
        tracing::error!(module = canonical_name, error = %e, "module schema literal is broken");
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
    let canonical_name = canonical_module_slug(&name).to_string();
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return Err((status, Json(ApiResponse::error("FORBIDDEN", msg))));
    }

    let (schema_value, schema) =
        resolve_schema(&canonical_name).map_err(|status| match status {
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

    let values = load_module_values(&state_guard.db, &canonical_name, &schema_value).await;
    drop(state_guard);

    Ok(Json(ModuleConfig { schema, values }))
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
        (status = 200, description = "Updated config", body = ModuleConfigResponseSchema),
        (status = 400, description = "Module has no config schema", body = ModuleConfigResponseSchema),
        (status = 403, description = "Admin access required", body = ModuleConfigResponseSchema),
        (status = 404, description = "Unknown module slug", body = ModuleConfigResponseSchema),
        (status = 422, description = "Validation failed", body = ModuleConfigResponseSchema)
    )
)]
pub async fn patch_module_config(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Path(name): Path<String>,
    Json(body): Json<UpdateModuleConfigRequest>,
) -> (StatusCode, Json<ApiResponse<ModuleConfig>>) {
    let canonical_name = canonical_module_slug(&name).to_string();
    let state_guard = state.read().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let (schema_value, schema) = match resolve_schema(&canonical_name) {
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
        let key = config_setting_key(&canonical_name, field);
        let encoded = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    module = %canonical_name,
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
                module = %canonical_name,
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
            .resource("module", &canonical_name)
            .details(serde_json::json!({
                "module": canonical_name,
                "keys_changed": keys_changed,
            }))
            .log();
        entry.persist(&state_guard.db).await;
    }

    // Re-read persisted values so the response reflects the new state
    // exactly as a subsequent GET would.
    let values = load_module_values(&state_guard.db, &canonical_name, &schema_value).await;
    drop(state_guard);

    (
        StatusCode::OK,
        Json(ApiResponse::success(ModuleConfig { schema, values })),
    )
}
