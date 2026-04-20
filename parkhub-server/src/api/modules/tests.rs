//! Tests for the module registry, runtime-toggle handlers, config editor,
//! and `module_gate` middleware.

#![cfg(test)]

use super::gate::{MODULE_ROUTES, module_for_path, module_gate};
use super::handlers::{
    get_module, get_module_config, list_modules, patch_admin_module, patch_module_config,
};
use super::registry::{module_registry_static, registry_defs};
use super::{
    AuthUser, ModuleCategory, SharedState, UpdateModuleConfigRequest, UpdateModuleRequest,
    config_setting_key, module_registry, runtime_enabled_setting_key,
};

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
};
use std::collections::{HashMap, HashSet};

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

/// Rust ships GDPR self-service endpoints and a Prometheus exporter;
/// both must be visible in the public module registry so the PHP and
/// Rust contracts stay aligned.
#[test]
fn test_registry_includes_gdpr_and_metrics_modules() {
    let registry = module_registry_static();

    let gdpr = registry
        .iter()
        .find(|m| m.name == "gdpr")
        .expect("registry missing gdpr module");
    assert_eq!(gdpr.category, ModuleCategory::Compliance);
    assert!(
        gdpr.enabled,
        "gdpr should be enabled in the static registry"
    );
    assert!(!gdpr.runtime_toggleable);

    let metrics = registry
        .iter()
        .find(|m| m.name == "metrics")
        .expect("registry missing metrics module");
    assert_eq!(metrics.category, ModuleCategory::Analytics);
    assert!(
        metrics.enabled,
        "metrics should be enabled in the static registry"
    );
    assert!(!metrics.runtime_toggleable);
}

/// Public admin surfaces that already ship in Rust must be visible in the
/// module contract so dashboards and module pickers can deep-link to them.
#[test]
fn test_registry_includes_admin_reports_audit_log_and_rate_dashboard_modules() {
    let registry = module_registry_static();

    let admin_reports = registry
        .iter()
        .find(|m| m.name == "admin-reports")
        .expect("registry missing admin-reports module");
    assert_eq!(admin_reports.category, ModuleCategory::Analytics);
    assert!(admin_reports.enabled);
    assert!(!admin_reports.runtime_toggleable);
    assert_eq!(admin_reports.ui_route.as_deref(), Some("/admin/reports"));

    let audit_log = registry
        .iter()
        .find(|m| m.name == "audit-log")
        .expect("registry missing audit-log module");
    assert_eq!(audit_log.category, ModuleCategory::Admin);
    assert!(audit_log.enabled);
    assert!(!audit_log.runtime_toggleable);
    assert_eq!(audit_log.ui_route.as_deref(), Some("/admin/audit-log"));

    let rate_dashboard = registry
        .iter()
        .find(|m| m.name == "rate-dashboard")
        .expect("registry missing rate-dashboard module");
    assert_eq!(rate_dashboard.category, ModuleCategory::Admin);
    assert!(rate_dashboard.enabled);
    assert!(!rate_dashboard.runtime_toggleable);
    assert_eq!(
        rate_dashboard.ui_route.as_deref(),
        Some("/admin/rate-limits")
    );
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
    assert!(
        response.success,
        "list_modules should return success envelope"
    );
    assert!(response.error.is_none());
    let response = response.data.expect("module payload");

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

    assert!(
        response.module_info.iter().any(|m| m.name == "gdpr"),
        "gdpr module must be present in module_info"
    );
    assert_eq!(response.modules.get("gdpr").copied(), Some(true));

    assert!(
        response.module_info.iter().any(|m| m.name == "metrics"),
        "metrics module must be present in module_info"
    );
    assert_eq!(response.modules.get("metrics").copied(), Some(true));

    for module in ["admin-reports", "audit-log", "rate-dashboard"] {
        assert!(
            response.module_info.iter().any(|m| m.name == module),
            "{module} module must be present in module_info"
        );
        assert_eq!(response.modules.get(module).copied(), Some(true));
    }
}

/// `get_module` returns 404 on unknown slugs.
#[tokio::test]
async fn test_get_module_unknown_returns_404() {
    let (_dir, state) = test_state();

    let (status, Json(response)) =
        get_module(State(state), Path("does-not-exist".to_string())).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(
        response.error.expect("error payload").code,
        "UNKNOWN_MODULE"
    );
}

/// Legacy transport-named slug lookups stay compatible, but the public
/// module contract emits the capability-first `realtime` name.
#[tokio::test]
async fn test_get_module_legacy_websocket_slug_returns_realtime_module() {
    let (_dir, state) = test_state();

    let (status, Json(response)) = get_module(State(state), Path("websocket".to_string())).await;
    assert_eq!(status, StatusCode::OK);
    assert!(response.success);
    let module = response.data.expect("module payload");
    assert_eq!(module.name, "realtime");
    assert_eq!(module.category, ModuleCategory::Integration);
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

/// Legacy transport slugs still resolve on the admin toggle surface.
/// `websocket` maps to the canonical `realtime` module and therefore
/// returns the realtime module's semantics (`409`, not `400`).
#[tokio::test]
async fn test_patch_legacy_websocket_alias_returns_realtime_toggle_semantics() {
    let (_dir, state) = test_state();
    let admin = seed_user(&state, UserRole::Admin).await;

    let (status, Json(response)) = patch_admin_module(
        State(state.clone()),
        axum::Extension(admin),
        Path("websocket".to_string()),
        Json(UpdateModuleRequest {
            runtime_enabled: false,
        }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert!(!response.success);
    assert_eq!(
        response.error.expect("error payload").code,
        "NOT_RUNTIME_TOGGLEABLE"
    );
    assert!(
        state
            .read()
            .await
            .db
            .get_setting(&runtime_enabled_setting_key("realtime"))
            .await
            .expect("get_setting")
            .is_none(),
        "legacy alias must not create a realtime runtime toggle setting"
    );
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

/// Legacy transport slugs stay compatible on the admin config read
/// path: `websocket` resolves to `realtime` and therefore returns
/// `400 NO_CONFIG_SCHEMA` instead of `404 UNKNOWN_MODULE`.
#[tokio::test]
async fn test_get_config_legacy_websocket_alias_returns_realtime_semantics() {
    let (_dir, state) = test_state();
    let admin = seed_user(&state, UserRole::Admin).await;

    let result = get_module_config(
        State(state),
        axum::Extension(admin),
        Path("websocket".to_string()),
    )
    .await;
    let err = result.expect_err("400");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        err.1.error.as_ref().expect("error payload").code,
        "NO_CONFIG_SCHEMA"
    );
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

/// Legacy transport slugs stay compatible on the admin config write
/// path: `websocket` resolves to `realtime` and therefore returns
/// `400 NO_CONFIG_SCHEMA` instead of `404 UNKNOWN_MODULE`.
#[tokio::test]
async fn test_patch_config_legacy_websocket_alias_returns_realtime_semantics() {
    let (_dir, state) = test_state();
    let admin = seed_user(&state, UserRole::Admin).await;

    let mut values = HashMap::new();
    values.insert(
        "default_theme".to_string(),
        serde_json::Value::String("dark".to_string()),
    );

    let (status, Json(response)) = patch_module_config(
        State(state),
        axum::Extension(admin),
        Path("websocket".to_string()),
        Json(UpdateModuleConfigRequest { values }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        response.error.expect("error payload").code,
        "NO_CONFIG_SCHEMA"
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
    let details: serde_json::Value = serde_json::from_str(details_raw).expect("details is JSON");
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
