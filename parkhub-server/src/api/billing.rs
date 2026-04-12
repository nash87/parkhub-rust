//! Cost Center Billing — endpoints for cost center and department billing
//!
//! - `GET  /api/v1/admin/billing/by-cost-center` — aggregate bookings/credits by cost center
//! - `GET  /api/v1/admin/billing/by-department` — aggregate by department
//! - `GET  /api/v1/admin/billing/export` — CSV export with cost center breakdown
//! - `POST /api/v1/admin/billing/allocate` — manual credit allocation per cost center

use axum::{
    Extension, Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;

use parkhub_common::{ApiResponse, BookingStatus};

use super::{AuthUser, check_admin};
use crate::AppState;

type SharedState = Arc<RwLock<AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Billing summary per cost center
#[derive(Debug, Clone, Serialize, Default)]
pub struct CostCenterSummary {
    pub cost_center: String,
    pub department: String,
    pub user_count: usize,
    pub total_bookings: usize,
    pub total_credits_used: i32,
    pub total_amount: f64,
    pub currency: String,
}

/// Billing summary per department
#[derive(Debug, Clone, Serialize, Default)]
pub struct DepartmentSummary {
    pub department: String,
    pub user_count: usize,
    pub total_bookings: usize,
    pub total_credits_used: i32,
    pub total_amount: f64,
    pub currency: String,
}

/// Credit allocation request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AllocateCreditsRequest {
    pub cost_center: String,
    pub credits: i32,
    #[allow(dead_code)]
    pub reason: Option<String>,
}

/// Allocation result
#[derive(Debug, Serialize)]
pub struct AllocationResult {
    pub cost_center: String,
    pub users_affected: usize,
    pub credits_per_user: i32,
    pub total_allocated: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/billing/by-cost-center
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/billing/by-cost-center` — aggregate by cost center
#[utoipa::path(get, path = "/api/v1/admin/billing/by-cost-center", tag = "Billing",
    summary = "Billing by cost center",
    description = "Aggregate bookings, credits, and spending by cost center. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Cost center summaries"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn billing_by_cost_center(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<CostCenterSummary>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut summaries: HashMap<String, CostCenterSummary> = HashMap::new();

    for user in &users {
        let cc = user
            .cost_center
            .as_deref()
            .unwrap_or("Unassigned")
            .to_string();
        let dept = user.department.as_deref().unwrap_or("").to_string();

        let entry = summaries
            .entry(cc.clone())
            .or_insert_with(|| CostCenterSummary {
                cost_center: cc,
                department: dept.clone(),
                currency: "EUR".to_string(),
                ..Default::default()
            });
        entry.user_count += 1;
        if !dept.is_empty() && entry.department.is_empty() {
            entry.department = dept;
        }

        let user_bookings: Vec<_> = bookings
            .iter()
            .filter(|b| {
                b.user_id == user.id
                    && (b.status == BookingStatus::Completed
                        || b.status == BookingStatus::Active
                        || b.status == BookingStatus::Confirmed)
            })
            .collect();

        entry.total_bookings += user_bookings.len();
        for b in &user_bookings {
            entry.total_amount += b.pricing.total;
        }

        // Credits used = quota - balance (rough estimate)
        let used = (user.credits_monthly_quota - user.credits_balance).max(0);
        entry.total_credits_used += used;
    }

    let mut result: Vec<CostCenterSummary> = summaries.into_values().collect();
    result.sort_by(|a, b| {
        b.total_amount
            .partial_cmp(&a.total_amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/billing/by-department
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/billing/by-department` — aggregate by department
#[utoipa::path(get, path = "/api/v1/admin/billing/by-department", tag = "Billing",
    summary = "Billing by department",
    description = "Aggregate bookings, credits, and spending by department. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Department summaries"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn billing_by_department(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<DepartmentSummary>>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut summaries: HashMap<String, DepartmentSummary> = HashMap::new();

    for user in &users {
        let dept = user
            .department
            .as_deref()
            .unwrap_or("Unassigned")
            .to_string();

        let entry = summaries
            .entry(dept.clone())
            .or_insert_with(|| DepartmentSummary {
                department: dept,
                currency: "EUR".to_string(),
                ..Default::default()
            });
        entry.user_count += 1;

        let user_bookings: Vec<_> = bookings
            .iter()
            .filter(|b| {
                b.user_id == user.id
                    && (b.status == BookingStatus::Completed
                        || b.status == BookingStatus::Active
                        || b.status == BookingStatus::Confirmed)
            })
            .collect();

        entry.total_bookings += user_bookings.len();
        for b in &user_bookings {
            entry.total_amount += b.pricing.total;
        }

        let used = (user.credits_monthly_quota - user.credits_balance).max(0);
        entry.total_credits_used += used;
    }

    let mut result: Vec<DepartmentSummary> = summaries.into_values().collect();
    result.sort_by(|a, b| {
        b.total_amount
            .partial_cmp(&a.total_amount)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/v1/admin/billing/export
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/billing/export` — CSV export
#[utoipa::path(get, path = "/api/v1/admin/billing/export", tag = "Billing",
    summary = "Export billing CSV",
    description = "Download billing data as CSV with cost center breakdown. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "CSV file"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn billing_export_csv(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> impl IntoResponse {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (
            status,
            [(header::CONTENT_TYPE, "application/json")],
            format!("{{\"error\":\"{msg}\"}}"),
        );
    }

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut csv =
        String::from("Username,Email,Cost Center,Department,Bookings,Credits Used,Amount (EUR)\n");

    for user in &users {
        let cc = user.cost_center.as_deref().unwrap_or("");
        let dept = user.department.as_deref().unwrap_or("");

        let user_bookings = bookings
            .iter()
            .filter(|b| {
                b.user_id == user.id
                    && (b.status == BookingStatus::Completed
                        || b.status == BookingStatus::Active
                        || b.status == BookingStatus::Confirmed)
            })
            .count();

        let amount: f64 = bookings
            .iter()
            .filter(|b| {
                b.user_id == user.id
                    && (b.status == BookingStatus::Completed
                        || b.status == BookingStatus::Active
                        || b.status == BookingStatus::Confirmed)
            })
            .map(|b| b.pricing.total)
            .sum();

        let credits_used = (user.credits_monthly_quota - user.credits_balance).max(0);

        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{:.2}",
            user.username, user.email, cc, dept, user_bookings, credits_used, amount
        );
    }

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/csv")], csv)
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/v1/admin/billing/allocate
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/v1/admin/billing/allocate` — allocate credits per cost center
#[utoipa::path(post, path = "/api/v1/admin/billing/allocate", tag = "Billing",
    summary = "Allocate credits by cost center",
    description = "Grant credits equally to all users in a cost center. Admin only.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Credits allocated"),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn billing_allocate(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AllocateCreditsRequest>,
) -> (StatusCode, Json<ApiResponse<AllocationResult>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    if req.credits <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_CREDITS",
                "Credits must be positive",
            )),
        );
    }

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let matching: Vec<_> = users
        .iter()
        .filter(|u| u.cost_center.as_deref() == Some(&req.cost_center))
        .collect();

    if matching.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NO_USERS",
                "No users found in this cost center",
            )),
        );
    }

    let total_users = matching.len();
    let mut allocated = 0i32;

    for user in &matching {
        let mut u = (*user).clone();
        u.credits_balance += req.credits;
        allocated += req.credits;
        if let Err(e) = state_guard.db.save_user(&u).await {
            tracing::error!("Failed to allocate credits to {}: {e}", u.username);
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(AllocationResult {
            cost_center: req.cost_center,
            users_affected: total_users,
            credits_per_user: req.credits,
            total_allocated: allocated,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_center_summary_default() {
        let s = CostCenterSummary::default();
        assert_eq!(s.user_count, 0);
        assert_eq!(s.total_bookings, 0);
        assert_eq!(s.total_credits_used, 0);
        assert_eq!(s.total_amount, 0.0);
    }

    #[test]
    fn test_cost_center_summary_serialization() {
        let s = CostCenterSummary {
            cost_center: "CC-100".to_string(),
            department: "Engineering".to_string(),
            user_count: 5,
            total_bookings: 20,
            total_credits_used: 100,
            total_amount: 250.50,
            currency: "EUR".to_string(),
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["cost_center"], "CC-100");
        assert_eq!(json["department"], "Engineering");
        assert_eq!(json["user_count"], 5);
        assert_eq!(json["total_amount"], 250.50);
    }

    #[test]
    fn test_department_summary_serialization() {
        let s = DepartmentSummary {
            department: "Marketing".to_string(),
            user_count: 3,
            total_bookings: 10,
            total_credits_used: 30,
            total_amount: 75.00,
            currency: "EUR".to_string(),
        };
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["department"], "Marketing");
        assert_eq!(json["total_bookings"], 10);
    }

    #[test]
    fn test_allocate_request_deserialization() {
        let json = r#"{"cost_center":"CC-100","credits":10,"reason":"Monthly allocation"}"#;
        let req: AllocateCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.cost_center, "CC-100");
        assert_eq!(req.credits, 10);
        assert_eq!(req.reason.as_deref(), Some("Monthly allocation"));
    }

    #[test]
    fn test_allocate_request_no_reason() {
        let json = r#"{"cost_center":"CC-200","credits":5}"#;
        let req: AllocateCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.cost_center, "CC-200");
        assert!(req.reason.is_none());
    }

    #[test]
    fn test_allocation_result_serialization() {
        let r = AllocationResult {
            cost_center: "CC-100".to_string(),
            users_affected: 5,
            credits_per_user: 10,
            total_allocated: 50,
        };
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["users_affected"], 5);
        assert_eq!(json["total_allocated"], 50);
    }
}
