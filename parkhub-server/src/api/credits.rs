//! Credits handlers: user balance, admin grant, refill, quota management.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, CreditTransaction, CreditTransactionType, UserRole};

use crate::audit::{AuditEntry, AuditEventType};
use super::{admin::AdminUserResponse, check_admin, AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Response for user credits endpoint
#[derive(Debug, Serialize)]
pub struct UserCreditsResponse {
    pub credits_balance: i32,
    pub credits_monthly_quota: i32,
    pub credits_last_refilled: Option<chrono::DateTime<Utc>>,
    pub recent_transactions: Vec<CreditTransaction>,
}

/// Request body for admin credit grant
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AdminGrantCreditsRequest {
    amount: i32,
    description: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/user/credits` — get current user's credit balance and history
#[utoipa::path(
    get,
    path = "/api/v1/user/credits",
    tag = "Credits",
    summary = "Get credit balance",
    description = "Returns the authenticated user's credit balance, monthly quota, and recent transactions.",
    responses(
        (status = 200, description = "User credit balance and recent transactions"),
        (status = 404, description = "User not found"),
    )
)]
#[tracing::instrument(skip(state), fields(user_id = %auth_user.user_id))]
pub async fn get_user_credits(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<UserCreditsResponse>>) {
    let state_guard = state.read().await;

    let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "User not found")),
        );
    };

    let transactions = state_guard
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default();
    drop(state_guard);

    // Return last 20 transactions
    let recent: Vec<CreditTransaction> = transactions.into_iter().take(20).collect();

    (
        StatusCode::OK,
        Json(ApiResponse::success(UserCreditsResponse {
            credits_balance: user.credits_balance,
            credits_monthly_quota: user.credits_monthly_quota,
            credits_last_refilled: user.credits_last_refilled,
            recent_transactions: recent,
        })),
    )
}

/// `POST /api/v1/admin/users/{id}/credits` — grant credits to a user (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/users/{id}/credits",
    tag = "Credits",
    summary = "Grant credits to a user",
    description = "Add credits to a user's balance. Amount must be 1-10000. Admin only.",
    params(("id" = String, Path, description = "Target user ID")),
    request_body = AdminGrantCreditsRequest,
    responses(
        (status = 200, description = "Credits granted successfully"),
        (status = 400, description = "Invalid amount"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
    )
)]
#[tracing::instrument(skip(state, req), fields(admin_id = %auth_user.user_id, target_user_id = %user_id, amount = req.amount))]
pub async fn admin_grant_credits(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
    Json(req): Json<AdminGrantCreditsRequest>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.write().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut target_user = match state_guard.db.get_user(&user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if req.amount < 1 || req.amount > 10000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                "Amount must be between 1 and 10000",
            )),
        );
    }

    target_user.credits_balance += req.amount;
    if let Err(e) = state_guard.db.save_user(&target_user).await {
        tracing::error!("Failed to save user credits: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update credits",
            )),
        );
    }

    let grant_amount = req.amount;
    let grant_description = req.description.clone();
    let tx = CreditTransaction {
        id: Uuid::new_v4(),
        user_id: target_user.id,
        booking_id: None,
        amount: req.amount,
        transaction_type: CreditTransactionType::Grant,
        description: req.description.or_else(|| Some("Admin grant".to_string())),
        granted_by: Some(auth_user.user_id),
        created_at: Utc::now(),
    };
    if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
        tracing::warn!("Failed to save credit grant transaction: {e}");
    }

    let audit = AuditEntry::new(AuditEventType::ConfigChanged)
        .user(auth_user.user_id, "admin")
        .resource("user_credits", &user_id)
        .details(serde_json::json!({
            "action": "grant_credits",
            "amount": grant_amount,
            "description": grant_description,
        }))
        .log();
    audit.persist(&state_guard.db).await;
    drop(state_guard);

    (StatusCode::OK, Json(ApiResponse::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_grant_credits_request_deserialize() {
        let json = r#"{"amount": 50, "description": "Bonus credits"}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, 50);
        assert_eq!(req.description.as_deref(), Some("Bonus credits"));
    }

    #[test]
    fn test_admin_grant_credits_request_without_description() {
        let json = r#"{"amount": 10}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, 10);
        assert!(req.description.is_none());
    }

    #[test]
    fn test_admin_grant_credits_request_negative_amount() {
        let json = r#"{"amount": -5}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, -5); // Deserializes fine, handler validates range
    }

    #[test]
    fn test_admin_grant_credits_request_zero_amount() {
        let json = r#"{"amount": 0}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, 0);
    }

    #[test]
    fn test_admin_grant_credits_request_missing_amount_fails() {
        let json = r#"{"description": "no amount"}"#;
        let result: Result<AdminGrantCreditsRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_grant_credits_request_empty_description() {
        let json = r#"{"amount": 1, "description": ""}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description.as_deref(), Some(""));
    }

    #[test]
    fn test_admin_grant_credits_request_large_amount() {
        let json = r#"{"amount": 10000}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, 10000);
    }

    #[test]
    fn test_admin_grant_credits_request_max_i32() {
        let json = r#"{"amount": 2147483647}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, i32::MAX);
    }

    #[test]
    fn test_admin_grant_credits_request_min_i32() {
        let json = r#"{"amount": -2147483648}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount, i32::MIN);
    }

    #[test]
    fn test_admin_grant_credits_request_rejects_float() {
        let json = r#"{"amount": 5.5}"#;
        let result: Result<AdminGrantCreditsRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_grant_credits_request_rejects_string_amount() {
        let json = r#"{"amount": "fifty"}"#;
        let result: Result<AdminGrantCreditsRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_admin_grant_credits_request_long_description() {
        let long_desc = "x".repeat(10000);
        let json = serde_json::json!({"amount": 100, "description": long_desc});
        let req: AdminGrantCreditsRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.description.unwrap().len(), 10000);
    }

    #[test]
    fn test_admin_grant_credits_request_null_description() {
        let json = r#"{"amount": 10, "description": null}"#;
        let req: AdminGrantCreditsRequest = serde_json::from_str(json).unwrap();
        assert!(req.description.is_none());
    }

    #[test]
    fn test_admin_grant_credits_amount_boundary_validation_logic() {
        // Boundary: 1 is valid, 0 is not
        let at_min: AdminGrantCreditsRequest = serde_json::from_str(r#"{"amount": 1}"#).unwrap();
        assert_eq!(at_min.amount, 1);
        assert!(at_min.amount >= 1 && at_min.amount <= 10000);

        let below_min: AdminGrantCreditsRequest = serde_json::from_str(r#"{"amount": 0}"#).unwrap();
        assert!(below_min.amount < 1);

        let at_max: AdminGrantCreditsRequest =
            serde_json::from_str(r#"{"amount": 10000}"#).unwrap();
        assert!(at_max.amount >= 1 && at_max.amount <= 10000);

        let above_max: AdminGrantCreditsRequest =
            serde_json::from_str(r#"{"amount": 10001}"#).unwrap();
        assert!(above_max.amount > 10000);
    }

    #[test]
    fn test_credit_transaction_type_serde() {
        use parkhub_common::CreditTransactionType;

        let grant: CreditTransactionType = serde_json::from_str(r#""grant""#).unwrap();
        assert_eq!(grant, CreditTransactionType::Grant);

        let deduction: CreditTransactionType = serde_json::from_str(r#""deduction""#).unwrap();
        assert_eq!(deduction, CreditTransactionType::Deduction);

        let refund: CreditTransactionType = serde_json::from_str(r#""refund""#).unwrap();
        assert_eq!(refund, CreditTransactionType::Refund);

        let monthly_refill: CreditTransactionType =
            serde_json::from_str(r#""monthly_refill""#).unwrap();
        assert_eq!(monthly_refill, CreditTransactionType::MonthlyRefill);

        let adjustment: CreditTransactionType = serde_json::from_str(r#""adjustment""#).unwrap();
        assert_eq!(adjustment, CreditTransactionType::Adjustment);
    }

    #[test]
    fn test_credit_transaction_type_unknown_variant() {
        use parkhub_common::CreditTransactionType;
        let result: Result<CreditTransactionType, _> = serde_json::from_str(r#""unknown""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_credit_transaction_full_serde() {
        use parkhub_common::{CreditTransaction, CreditTransactionType};

        let tx_json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "user_id": "550e8400-e29b-41d4-a716-446655440001",
            "booking_id": null,
            "amount": 50,
            "transaction_type": "grant",
            "description": "Test grant",
            "granted_by": "550e8400-e29b-41d4-a716-446655440002",
            "created_at": "2024-01-01T00:00:00Z"
        });

        let tx: CreditTransaction = serde_json::from_value(tx_json).unwrap();
        assert_eq!(tx.amount, 50);
        assert_eq!(tx.transaction_type, CreditTransactionType::Grant);
        assert_eq!(tx.description.as_deref(), Some("Test grant"));
        assert!(tx.booking_id.is_none());
        assert!(tx.granted_by.is_some());
    }

    #[test]
    fn test_credit_transaction_with_booking_id() {
        use parkhub_common::{CreditTransaction, CreditTransactionType};

        let tx_json = serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "user_id": "550e8400-e29b-41d4-a716-446655440001",
            "booking_id": "550e8400-e29b-41d4-a716-446655440010",
            "amount": -30,
            "transaction_type": "deduction",
            "description": null,
            "granted_by": null,
            "created_at": "2024-06-15T12:00:00Z"
        });

        let tx: CreditTransaction = serde_json::from_value(tx_json).unwrap();
        assert_eq!(tx.amount, -30);
        assert_eq!(tx.transaction_type, CreditTransactionType::Deduction);
        assert!(tx.booking_id.is_some());
        assert!(tx.granted_by.is_none());
        assert!(tx.description.is_none());
    }
}

/// `POST /api/v1/admin/credits/refill-all` — refill all active users' credits (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/admin/credits/refill-all",
    tag = "Credits",
    summary = "Refill all users' credits",
    description = "Reset all active non-admin users' credit balances to their monthly quota. Admin only.",
    responses(
        (status = 200, description = "All active users refilled"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn admin_refill_all_credits(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.write().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to list users")),
            );
        }
    };

    let mut refilled = 0;
    let now = Utc::now();
    for mut user in users {
        if !user.is_active {
            continue;
        }
        if user.role == UserRole::Admin || user.role == UserRole::SuperAdmin {
            continue;
        }
        let old_balance = user.credits_balance;
        user.credits_balance = user.credits_monthly_quota;
        user.credits_last_refilled = Some(now);
        if matches!(state_guard.db.save_user(&user).await, Ok(())) {
            let tx = CreditTransaction {
                id: Uuid::new_v4(),
                user_id: user.id,
                booking_id: None,
                amount: user.credits_monthly_quota - old_balance,
                transaction_type: CreditTransactionType::MonthlyRefill,
                description: Some("Monthly refill".to_string()),
                granted_by: Some(auth_user.user_id),
                created_at: now,
            };
            if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
                tracing::warn!(
                    "Failed to save refill transaction for user {}: {e}",
                    user.id
                );
            }
            refilled += 1;
        }
    }
    drop(state_guard);

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::json!({ "users_refilled": refilled }),
        )),
    )
}

/// `PUT /api/v1/admin/users/{id}/quota` — update a user's monthly credit quota (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/admin/users/{id}/quota",
    tag = "Credits",
    summary = "Update user's monthly quota",
    description = "Set a user's monthly credit allowance (0-999). Admin only.",
    params(("id" = String, Path, description = "Target user ID")),
    responses(
        (status = 200, description = "Quota updated successfully"),
        (status = 400, description = "Invalid quota value"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "User not found"),
    )
)]
pub async fn admin_update_user_quota(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(user_id): Path<String>,
    Json(req): Json<crate::requests::UpdateQuotaRequest>,
) -> (StatusCode, Json<ApiResponse<AdminUserResponse>>) {
    use validator::Validate;

    if let Err(e) = req.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "VALIDATION_ERROR",
                format!("Invalid quota: {e}"),
            )),
        );
    }

    let state_guard = state.write().await;

    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let mut target_user = match state_guard.db.get_user(&user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let old_quota = target_user.credits_monthly_quota;
    target_user.credits_monthly_quota = req.monthly_quota;
    target_user.updated_at = Utc::now();

    if let Err(e) = state_guard.db.save_user(&target_user).await {
        tracing::error!("Failed to save user quota: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update quota")),
        );
    }

    // Log quota change as an Adjustment transaction
    let tx = CreditTransaction {
        id: Uuid::new_v4(),
        user_id: target_user.id,
        booking_id: None,
        amount: req.monthly_quota - old_quota,
        transaction_type: CreditTransactionType::Adjustment,
        description: Some(format!(
            "Quota changed from {} to {} by admin",
            old_quota, req.monthly_quota
        )),
        granted_by: Some(auth_user.user_id),
        created_at: Utc::now(),
    };
    if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
        tracing::warn!("Failed to save quota adjustment transaction: {e}");
    }
    drop(state_guard);

    tracing::info!(
        "Admin {} updated quota for user {} from {} to {}",
        auth_user.user_id,
        target_user.id,
        old_quota,
        req.monthly_quota
    );

    (
        StatusCode::OK,
        Json(ApiResponse::success(AdminUserResponse::from(&target_user))),
    )
}
