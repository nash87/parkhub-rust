//! Runtime module gating middleware.
//!
//! A request whose path matches the route prefix of a runtime-disabled
//! module is short-circuited with `404 NOT_FOUND` — indistinguishable
//! from a feature that was never compiled in. Keeps the failure mode
//! uniform across the two disable paths (feature flag vs. runtime
//! toggle).

#![allow(clippy::significant_drop_tightening)]

use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use parkhub_common::ApiResponse;

use super::{SharedState, module_registry};

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
