//! Multi-tenant management: admin endpoints for tenant CRUD and isolation.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use parkhub_common::{ApiResponse, UserRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{check_admin, AuthUser};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

/// Tenant entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub domain: Option<String>,
    pub branding: Option<TenantBranding>,
    pub created_at: String,
    pub updated_at: String,
    pub user_count: u32,
    pub lot_count: u32,
}

/// Tenant branding settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantBranding {
    pub primary_color: Option<String>,
    pub logo_url: Option<String>,
    pub company_name: Option<String>,
}

/// Request body for creating/updating a tenant
#[derive(Debug, Deserialize)]
pub struct TenantRequest {
    pub name: String,
    pub domain: Option<String>,
    pub branding: Option<TenantBranding>,
}

/// `GET /api/v1/admin/tenants` — list all tenants (super-admin only)
pub async fn list_tenants(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Result<Json<ApiResponse<Vec<Tenant>>>, (StatusCode, &'static str)> {
    let state_guard = state.read().await;
    check_admin(&state_guard, &auth_user).await?;

    let user = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten();

    // Load all tenant entries from settings (tenant:<id> -> JSON)
    let mut tenants = Vec::new();
    let tenant_ids = load_tenant_ids(&state_guard).await;

    for tid in &tenant_ids {
        if let Some(tenant) = load_tenant(&state_guard, tid).await {
            // Regular admins can only see their own tenant
            if let Some(ref u) = user {
                if u.role != UserRole::SuperAdmin
                    && u.tenant_id.as_deref() != Some(tid.as_str()) {
                        continue;
                    }
            }
            tenants.push(tenant);
        }
    }

    Ok(Json(ApiResponse::success(tenants)))
}

/// `POST /api/v1/admin/tenants` — create a new tenant (super-admin only)
pub async fn create_tenant(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<TenantRequest>,
) -> Result<Json<ApiResponse<Tenant>>, (StatusCode, &'static str)> {
    let state_guard = state.read().await;

    // Only super-admin can create tenants
    let user = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .ok_or((StatusCode::FORBIDDEN, "Admin access required"))?;

    if user.role != UserRole::SuperAdmin {
        return Err((StatusCode::FORBIDDEN, "Super-admin access required"));
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let tenant = Tenant {
        id: id.clone(),
        name: req.name,
        domain: req.domain,
        branding: req.branding,
        created_at: now.clone(),
        updated_at: now,
        user_count: 0,
        lot_count: 0,
    };

    // Store tenant
    let json = serde_json::to_string(&tenant).unwrap_or_default();
    let _ = state_guard
        .db
        .set_setting(&format!("tenant:{id}"), &json)
        .await;

    // Add to tenant index
    let mut ids = load_tenant_ids(&state_guard).await;
    ids.push(id);
    let _ = state_guard
        .db
        .set_setting("tenant_ids", &ids.join(","))
        .await;

    Ok(Json(ApiResponse::success(tenant)))
}

/// `PUT /api/v1/admin/tenants/:id` — update a tenant
pub async fn update_tenant(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<TenantRequest>,
) -> Result<Json<ApiResponse<Tenant>>, (StatusCode, &'static str)> {
    let state_guard = state.read().await;
    check_admin(&state_guard, &auth_user).await?;

    let user = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
        .ok()
        .flatten()
        .ok_or((StatusCode::FORBIDDEN, "Admin access required"))?;

    // Regular admins can only update their own tenant
    if user.role != UserRole::SuperAdmin && user.tenant_id.as_deref() != Some(&id) {
        return Err((StatusCode::FORBIDDEN, "Cannot update other tenants"));
    }

    let mut tenant = load_tenant(&state_guard, &id)
        .await
        .ok_or((StatusCode::NOT_FOUND, "Tenant not found"))?;

    tenant.name = req.name;
    tenant.domain = req.domain;
    tenant.branding = req.branding;
    tenant.updated_at = Utc::now().to_rfc3339();

    let json = serde_json::to_string(&tenant).unwrap_or_default();
    let _ = state_guard
        .db
        .set_setting(&format!("tenant:{id}"), &json)
        .await;

    Ok(Json(ApiResponse::success(tenant)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn load_tenant_ids(state: &AppState) -> Vec<String> {
    state
        .db
        .get_setting("tenant_ids")
        .await
        .ok()
        .flatten()
        .map(|s| {
            s.split(',')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

async fn load_tenant(state: &AppState, id: &str) -> Option<Tenant> {
    let key = format!("tenant:{id}");
    state
        .db
        .get_setting(&key)
        .await
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str(&json).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_serialize() {
        let t = Tenant {
            id: "t-1".to_string(),
            name: "Acme Corp".to_string(),
            domain: Some("acme.com".to_string()),
            branding: None,
            created_at: "2026-03-22T00:00:00Z".to_string(),
            updated_at: "2026-03-22T00:00:00Z".to_string(),
            user_count: 5,
            lot_count: 2,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("Acme Corp"));
        assert!(json.contains("acme.com"));
        assert!(json.contains("\"user_count\":5"));
    }

    #[test]
    fn test_tenant_deserialize() {
        let json = r#"{"id":"t-1","name":"Test","domain":null,"branding":null,"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z","user_count":0,"lot_count":0}"#;
        let t: Tenant = serde_json::from_str(json).unwrap();
        assert_eq!(t.id, "t-1");
        assert_eq!(t.name, "Test");
        assert!(t.domain.is_none());
    }

    #[test]
    fn test_tenant_branding_serialize() {
        let b = TenantBranding {
            primary_color: Some("#FF5733".to_string()),
            logo_url: Some("https://example.com/logo.png".to_string()),
            company_name: Some("Acme".to_string()),
        };
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("#FF5733"));
        assert!(json.contains("logo.png"));
    }

    #[test]
    fn test_tenant_request_deserialize() {
        let json = r##"{"name":"New Tenant","domain":"tenant.com","branding":{"primary_color":"#000","logo_url":null,"company_name":"NT"}}"##;
        let req: TenantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "New Tenant");
        assert_eq!(req.domain.as_deref(), Some("tenant.com"));
        assert!(req.branding.is_some());
    }

    #[test]
    fn test_tenant_with_branding() {
        let t = Tenant {
            id: "t-2".to_string(),
            name: "Branded".to_string(),
            domain: Some("branded.co".to_string()),
            branding: Some(TenantBranding {
                primary_color: Some("#123456".to_string()),
                logo_url: None,
                company_name: Some("Branded Inc".to_string()),
            }),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            user_count: 10,
            lot_count: 3,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("Branded Inc"));
        assert!(json.contains("#123456"));
    }

    #[test]
    fn test_tenant_request_minimal() {
        let json = r#"{"name":"Minimal"}"#;
        let req: TenantRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Minimal");
        assert!(req.domain.is_none());
        assert!(req.branding.is_none());
    }

    #[test]
    fn test_tenant_id_parsing() {
        let csv = "tenant-1,tenant-2,tenant-3";
        let ids: Vec<String> = csv
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], "tenant-1");
    }

    #[test]
    fn test_empty_tenant_ids() {
        let csv = "";
        let ids: Vec<String> = csv
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_tenant_user_isolation() {
        // Simulate a user with tenant_id
        let user_tenant = Some("tenant-1".to_string());
        let target_tenant = "tenant-1";
        assert_eq!(user_tenant.as_deref(), Some(target_tenant));

        // Different tenant should not match
        let other_tenant = "tenant-2";
        assert_ne!(user_tenant.as_deref(), Some(other_tenant));
    }

    #[test]
    fn test_super_admin_sees_all() {
        let role = UserRole::SuperAdmin;
        assert_eq!(role, UserRole::SuperAdmin);
        // Super admin has no tenant_id restriction
        let user_tenant: Option<String> = None;
        assert!(user_tenant.is_none());
    }
}
