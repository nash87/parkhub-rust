//! Role-Based Access Control (RBAC) — Fine-grained permission management.
//!
//! Provides a flexible role & permission system beyond the built-in User/Admin/SuperAdmin
//! hierarchy.  Custom roles can be created with any combination of permissions.
//!
//! Endpoints:
//! - `GET    /api/v1/admin/roles`              — list all roles with permissions
//! - `POST   /api/v1/admin/roles`              — create custom role
//! - `PUT    /api/v1/admin/roles/{id}`          — update role permissions
//! - `DELETE /api/v1/admin/roles/{id}`          — delete custom role
//! - `GET    /api/v1/admin/users/{id}/roles`    — get user role assignments
//! - `PUT    /api/v1/admin/users/{id}/roles`    — assign roles to user

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use crate::audit::{AuditEntry, AuditEventType};

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// All available permissions in ParkHub.
pub const ALL_PERMISSIONS: &[&str] = &[
    "manage_users",
    "manage_lots",
    "manage_bookings",
    "view_reports",
    "manage_settings",
    "manage_plugins",
];

/// Built-in role names that cannot be deleted.
#[allow(dead_code)]
pub const BUILT_IN_ROLES: &[&str] = &["super_admin", "admin", "manager", "user", "viewer"];

/// A role definition with associated permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRole {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub built_in: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Role assignment linking a user to one or more roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleAssignment {
    pub user_id: Uuid,
    pub roles: Vec<RbacRoleSummary>,
}

/// Lightweight role summary for user assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacRoleSummary {
    pub id: Uuid,
    pub name: String,
    pub permissions: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AssignRolesRequest {
    pub role_ids: Vec<Uuid>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Default built-in roles with their permission sets.
pub fn default_roles() -> Vec<RbacRole> {
    let now = Utc::now();
    vec![
        RbacRole {
            id: Uuid::new_v4(),
            name: "super_admin".to_string(),
            description: Some(
                "Full system access including user management and settings".to_string(),
            ),
            permissions: ALL_PERMISSIONS.iter().map(|s| (*s).to_string()).collect(),
            built_in: true,
            created_at: now,
            updated_at: now,
        },
        RbacRole {
            id: Uuid::new_v4(),
            name: "admin".to_string(),
            description: Some("Administrative access to lots, bookings, and reports".to_string()),
            permissions: vec![
                "manage_lots".to_string(),
                "manage_bookings".to_string(),
                "view_reports".to_string(),
                "manage_settings".to_string(),
            ],
            built_in: true,
            created_at: now,
            updated_at: now,
        },
        RbacRole {
            id: Uuid::new_v4(),
            name: "manager".to_string(),
            description: Some("Manage bookings and view reports".to_string()),
            permissions: vec!["manage_bookings".to_string(), "view_reports".to_string()],
            built_in: true,
            created_at: now,
            updated_at: now,
        },
        RbacRole {
            id: Uuid::new_v4(),
            name: "user".to_string(),
            description: Some("Standard user with booking access".to_string()),
            permissions: vec![],
            built_in: true,
            created_at: now,
            updated_at: now,
        },
        RbacRole {
            id: Uuid::new_v4(),
            name: "viewer".to_string(),
            description: Some("Read-only access to reports".to_string()),
            permissions: vec!["view_reports".to_string()],
            built_in: true,
            created_at: now,
            updated_at: now,
        },
    ]
}

/// Validate that all permissions in a list are recognized.
pub fn validate_permissions(permissions: &[String]) -> Option<String> {
    let unknown: Vec<_> = permissions
        .iter()
        .filter(|p| !ALL_PERMISSIONS.contains(&p.as_str()))
        .collect();
    if unknown.is_empty() {
        None
    } else {
        Some(format!(
            "Unknown permissions: {}",
            unknown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

/// Check if a user has the required permission via their RBAC roles.
#[allow(dead_code)]
pub fn has_permission(roles: &[RbacRole], permission: &str) -> bool {
    roles
        .iter()
        .any(|r| r.permissions.iter().any(|p| p == permission))
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings key helpers (roles stored in admin settings as JSON)
// ─────────────────────────────────────────────────────────────────────────────

const RBAC_ROLES_KEY: &str = "rbac_roles";
const RBAC_USER_ROLES_PREFIX: &str = "rbac_user_roles_";

async fn load_roles(state: &crate::AppState) -> Vec<RbacRole> {
    match state.db.get_setting(RBAC_ROLES_KEY).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_else(|_| default_roles()),
        _ => default_roles(),
    }
}

async fn save_roles(state: &crate::AppState, roles: &[RbacRole]) -> Result<(), String> {
    let json = serde_json::to_string(roles).map_err(|e| e.to_string())?;
    state
        .db
        .set_setting(RBAC_ROLES_KEY, &json)
        .await
        .map_err(|e| e.to_string())
}

async fn load_user_role_ids(state: &crate::AppState, user_id: &str) -> Vec<Uuid> {
    let key = format!("{RBAC_USER_ROLES_PREFIX}{user_id}");
    match state.db.get_setting(&key).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => vec![],
    }
}

async fn save_user_role_ids(
    state: &crate::AppState,
    user_id: &str,
    role_ids: &[Uuid],
) -> Result<(), String> {
    let key = format!("{RBAC_USER_ROLES_PREFIX}{user_id}");
    let json = serde_json::to_string(role_ids).map_err(|e| e.to_string())?;
    state
        .db
        .set_setting(&key, &json)
        .await
        .map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/roles` — list all roles with permissions.
#[utoipa::path(
    get,
    path = "/api/v1/admin/roles",
    tag = "RBAC",
    summary = "List all roles",
    description = "Returns all defined roles including built-in and custom roles with their permissions.",
    responses(
        (status = 200, description = "List of roles"),
    )
)]
pub async fn list_roles(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<RbacRole>>> {
    let state_guard = state.read().await;
    let roles = load_roles(&state_guard).await;
    Json(ApiResponse::success(roles))
}

/// `POST /api/v1/admin/roles` — create a custom role.
#[utoipa::path(
    post,
    path = "/api/v1/admin/roles",
    tag = "RBAC",
    summary = "Create custom role",
    description = "Create a new custom role with specified permissions. Only super_admin can create roles.",
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created"),
        (status = 400, description = "Invalid permissions"),
        (status = 409, description = "Role name already exists"),
    )
)]
pub async fn create_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateRoleRequest>,
) -> (StatusCode, Json<ApiResponse<RbacRole>>) {
    let state_guard = state.read().await;

    // SuperAdmin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(
                    "FORBIDDEN",
                    "Super-admin access required",
                )),
            );
        }
    }

    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_NAME", "Role name is required")),
        );
    }

    if let Some(err) = validate_permissions(&req.permissions) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_PERMISSIONS", &err)),
        );
    }

    let mut roles = load_roles(&state_guard).await;

    // Check for duplicate name
    if roles.iter().any(|r| r.name.eq_ignore_ascii_case(&req.name)) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "DUPLICATE_NAME",
                "A role with this name already exists",
            )),
        );
    }

    let now = Utc::now();
    let role = RbacRole {
        id: Uuid::new_v4(),
        name: req.name.trim().to_string(),
        description: req.description,
        permissions: req.permissions,
        built_in: false,
        created_at: now,
        updated_at: now,
    };

    roles.push(role.clone());

    if let Err(e) = save_roles(&state_guard, &roles).await {
        tracing::error!("Failed to save roles: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save role")),
        );
    }

    // Audit
    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("rbac_role_created:{}", role.name))
        .log()
        .persist(&state_guard.db)
        .await;

    (StatusCode::CREATED, Json(ApiResponse::success(role)))
}

/// `PUT /api/v1/admin/roles/{id}` — update role permissions.
#[utoipa::path(
    put,
    path = "/api/v1/admin/roles/{id}",
    tag = "RBAC",
    summary = "Update role",
    description = "Update an existing role's name, description, or permissions.",
    params(("id" = String, Path, description = "Role ID")),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated"),
        (status = 400, description = "Invalid permissions"),
        (status = 404, description = "Role not found"),
    )
)]
pub async fn update_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(role_id): Path<String>,
    Json(req): Json<UpdateRoleRequest>,
) -> (StatusCode, Json<ApiResponse<RbacRole>>) {
    let state_guard = state.read().await;

    // SuperAdmin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(
                    "FORBIDDEN",
                    "Super-admin access required",
                )),
            );
        }
    }

    let id = match Uuid::parse_str(&role_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid role ID")),
            );
        }
    };

    if let Some(ref perms) = req.permissions {
        if let Some(err) = validate_permissions(perms) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_PERMISSIONS", &err)),
            );
        }
    }

    let mut roles = load_roles(&state_guard).await;
    let Some(role) = roles.iter_mut().find(|r| r.id == id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Role not found")),
        );
    };

    if let Some(name) = req.name {
        role.name = name;
    }
    if let Some(desc) = req.description {
        role.description = Some(desc);
    }
    if let Some(perms) = req.permissions {
        role.permissions = perms;
    }
    role.updated_at = Utc::now();

    let updated = role.clone();

    if let Err(e) = save_roles(&state_guard, &roles).await {
        tracing::error!("Failed to save roles: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save role")),
        );
    }

    // Audit
    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("rbac_role_updated:{}", updated.name))
        .log()
        .persist(&state_guard.db)
        .await;

    (StatusCode::OK, Json(ApiResponse::success(updated)))
}

/// `DELETE /api/v1/admin/roles/{id}` — delete a custom role.
#[utoipa::path(
    delete,
    path = "/api/v1/admin/roles/{id}",
    tag = "RBAC",
    summary = "Delete custom role",
    description = "Delete a custom role. Built-in roles cannot be deleted.",
    params(("id" = String, Path, description = "Role ID")),
    responses(
        (status = 200, description = "Role deleted"),
        (status = 400, description = "Cannot delete built-in role"),
        (status = 404, description = "Role not found"),
    )
)]
pub async fn delete_role(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(role_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // SuperAdmin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(
                    "FORBIDDEN",
                    "Super-admin access required",
                )),
            );
        }
    }

    let id = match Uuid::parse_str(&role_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid role ID")),
            );
        }
    };

    let mut roles = load_roles(&state_guard).await;
    let Some(idx) = roles.iter().position(|r| r.id == id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Role not found")),
        );
    };

    if roles[idx].built_in {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "BUILT_IN",
                "Cannot delete built-in roles",
            )),
        );
    }

    let removed = roles.remove(idx);

    if let Err(e) = save_roles(&state_guard, &roles).await {
        tracing::error!("Failed to save roles: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save roles")),
        );
    }

    // Audit
    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!("rbac_role_deleted:{}", removed.name))
        .log()
        .persist(&state_guard.db)
        .await;

    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// `GET /api/v1/admin/users/{id}/roles` — get user role assignments.
#[utoipa::path(
    get,
    path = "/api/v1/admin/users/{id}/roles",
    tag = "RBAC",
    summary = "Get user roles",
    description = "Returns all roles assigned to a specific user.",
    params(("id" = String, Path, description = "User ID")),
    responses(
        (status = 200, description = "User role assignments"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn get_user_roles(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<UserRoleAssignment>>) {
    let state_guard = state.read().await;

    // Verify user exists
    let uid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid user ID")),
            );
        }
    };

    match state_guard.db.get_user(&user_id).await {
        Ok(Some(_)) => {}
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    }

    let all_roles = load_roles(&state_guard).await;
    let user_role_ids = load_user_role_ids(&state_guard, &user_id).await;

    let assigned: Vec<RbacRoleSummary> = all_roles
        .iter()
        .filter(|r| user_role_ids.contains(&r.id))
        .map(|r| RbacRoleSummary {
            id: r.id,
            name: r.name.clone(),
            permissions: r.permissions.clone(),
        })
        .collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(UserRoleAssignment {
            user_id: uid,
            roles: assigned,
        })),
    )
}

/// `PUT /api/v1/admin/users/{id}/roles` — assign roles to a user.
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/roles",
    tag = "RBAC",
    summary = "Assign roles to user",
    description = "Replace a user's role assignments with the specified set of roles.",
    params(("id" = String, Path, description = "User ID")),
    request_body = AssignRolesRequest,
    responses(
        (status = 200, description = "Roles assigned"),
        (status = 400, description = "Invalid role IDs"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn assign_user_roles(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
    Json(req): Json<AssignRolesRequest>,
) -> (StatusCode, Json<ApiResponse<UserRoleAssignment>>) {
    let state_guard = state.read().await;

    // SuperAdmin check
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error(
                    "FORBIDDEN",
                    "Super-admin access required",
                )),
            );
        }
    }

    let uid = match Uuid::parse_str(&user_id) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_ID", "Invalid user ID")),
            );
        }
    };

    // Verify user exists
    match state_guard.db.get_user(&user_id).await {
        Ok(Some(_)) => {}
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    }

    let all_roles = load_roles(&state_guard).await;

    // Validate all role_ids exist
    let unknown: Vec<_> = req
        .role_ids
        .iter()
        .filter(|id| !all_roles.iter().any(|r| r.id == **id))
        .collect();

    if !unknown.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_ROLE_IDS",
                "One or more role IDs do not exist",
            )),
        );
    }

    if let Err(e) = save_user_role_ids(&state_guard, &user_id, &req.role_ids).await {
        tracing::error!("Failed to save user roles: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save roles")),
        );
    }

    let assigned: Vec<RbacRoleSummary> = all_roles
        .iter()
        .filter(|r| req.role_ids.contains(&r.id))
        .map(|r| RbacRoleSummary {
            id: r.id,
            name: r.name.clone(),
            permissions: r.permissions.clone(),
        })
        .collect();

    // Audit
    let role_names: Vec<_> = assigned.iter().map(|r| r.name.as_str()).collect();
    AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .detail(&format!(
            "rbac_roles_assigned:{}:{}",
            user_id,
            role_names.join(",")
        ))
        .log()
        .persist(&state_guard.db)
        .await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(UserRoleAssignment {
            user_id: uid,
            roles: assigned,
        })),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// RBAC permission middleware
// ─────────────────────────────────────────────────────────────────────────────

/// Check whether the authenticated user has a specific RBAC permission.
///
/// Returns `Ok(())` if the user has the permission (via any assigned role),
/// or if the user is a `SuperAdmin` (always has all permissions).
#[allow(dead_code)]
pub async fn check_rbac_permission(
    state: &crate::AppState,
    auth_user: &AuthUser,
    permission: &str,
) -> Result<(), (StatusCode, String)> {
    // SuperAdmin bypasses all checks
    if let Ok(Some(u)) = state.db.get_user(&auth_user.user_id.to_string()).await {
        if u.role == UserRole::SuperAdmin {
            return Ok(());
        }
    }

    let all_roles = load_roles(state).await;
    let user_role_ids = load_user_role_ids(state, &auth_user.user_id.to_string()).await;

    let user_roles: Vec<_> = all_roles
        .iter()
        .filter(|r| user_role_ids.contains(&r.id))
        .cloned()
        .collect();

    if has_permission(&user_roles, permission) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            format!("Missing permission: {permission}"),
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_roles_count() {
        let roles = default_roles();
        assert_eq!(roles.len(), 5);
    }

    #[test]
    fn test_default_roles_names() {
        let roles = default_roles();
        let names: Vec<_> = roles.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"super_admin"));
        assert!(names.contains(&"admin"));
        assert!(names.contains(&"manager"));
        assert!(names.contains(&"user"));
        assert!(names.contains(&"viewer"));
    }

    #[test]
    fn test_super_admin_has_all_permissions() {
        let roles = default_roles();
        let sa = roles.iter().find(|r| r.name == "super_admin").unwrap();
        for perm in ALL_PERMISSIONS {
            assert!(sa.permissions.contains(&(*perm).to_string()));
        }
    }

    #[test]
    fn test_viewer_has_only_view_reports() {
        let roles = default_roles();
        let viewer = roles.iter().find(|r| r.name == "viewer").unwrap();
        assert_eq!(viewer.permissions.len(), 1);
        assert_eq!(viewer.permissions[0], "view_reports");
    }

    #[test]
    fn test_user_has_no_permissions() {
        let roles = default_roles();
        let user = roles.iter().find(|r| r.name == "user").unwrap();
        assert!(user.permissions.is_empty());
    }

    #[test]
    fn test_validate_permissions_valid() {
        let perms = vec!["manage_users".to_string(), "view_reports".to_string()];
        assert!(validate_permissions(&perms).is_none());
    }

    #[test]
    fn test_validate_permissions_invalid() {
        let perms = vec!["manage_users".to_string(), "fly_rockets".to_string()];
        let err = validate_permissions(&perms);
        assert!(err.is_some());
        assert!(err.unwrap().contains("fly_rockets"));
    }

    #[test]
    fn test_validate_permissions_empty() {
        let perms: Vec<String> = vec![];
        assert!(validate_permissions(&perms).is_none());
    }

    #[test]
    fn test_has_permission_true() {
        let roles = default_roles();
        let admin_roles: Vec<_> = roles
            .iter()
            .filter(|r| r.name == "admin")
            .cloned()
            .collect();
        assert!(has_permission(&admin_roles, "manage_lots"));
    }

    #[test]
    fn test_has_permission_false() {
        let roles = default_roles();
        let viewer_roles: Vec<_> = roles
            .iter()
            .filter(|r| r.name == "viewer")
            .cloned()
            .collect();
        assert!(!has_permission(&viewer_roles, "manage_users"));
    }

    #[test]
    fn test_rbac_role_serialization() {
        let role = RbacRole {
            id: Uuid::new_v4(),
            name: "test_role".to_string(),
            description: Some("Test".to_string()),
            permissions: vec!["manage_users".to_string()],
            built_in: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&role).unwrap();
        let de: RbacRole = serde_json::from_str(&json).unwrap();
        assert_eq!(de.name, "test_role");
        assert!(!de.built_in);
    }

    #[test]
    fn test_user_role_assignment_serialization() {
        let assignment = UserRoleAssignment {
            user_id: Uuid::new_v4(),
            roles: vec![RbacRoleSummary {
                id: Uuid::new_v4(),
                name: "admin".to_string(),
                permissions: vec!["manage_lots".to_string()],
            }],
        };
        let json = serde_json::to_string(&assignment).unwrap();
        let de: UserRoleAssignment = serde_json::from_str(&json).unwrap();
        assert_eq!(de.roles.len(), 1);
        assert_eq!(de.roles[0].name, "admin");
    }

    #[test]
    fn test_built_in_roles_are_marked() {
        let roles = default_roles();
        for role in &roles {
            assert!(role.built_in);
        }
    }

    #[test]
    fn test_all_permissions_constant() {
        assert_eq!(ALL_PERMISSIONS.len(), 6);
        assert!(ALL_PERMISSIONS.contains(&"manage_users"));
        assert!(ALL_PERMISSIONS.contains(&"manage_lots"));
        assert!(ALL_PERMISSIONS.contains(&"manage_bookings"));
        assert!(ALL_PERMISSIONS.contains(&"view_reports"));
        assert!(ALL_PERMISSIONS.contains(&"manage_settings"));
        assert!(ALL_PERMISSIONS.contains(&"manage_plugins"));
    }

    #[test]
    fn test_manager_permissions() {
        let roles = default_roles();
        let mgr = roles.iter().find(|r| r.name == "manager").unwrap();
        assert_eq!(mgr.permissions.len(), 2);
        assert!(mgr.permissions.contains(&"manage_bookings".to_string()));
        assert!(mgr.permissions.contains(&"view_reports".to_string()));
    }
}
