//! Credits handlers: user balance, admin grant, refill, quota management.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{
    ApiResponse, CreditTransaction, CreditTransactionType, UserRole,
};

use super::{admin::AdminUserResponse, check_admin, AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Response for user credits endpoint
#[derive(Debug, Serialize)]
pub(crate) struct UserCreditsResponse {
    pub credits_balance: i32,
    pub credits_monthly_quota: i32,
    pub credits_last_refilled: Option<chrono::DateTime<Utc>>,
    pub recent_transactions: Vec<CreditTransaction>,
}

/// Request body for admin credit grant
#[derive(Debug, Deserialize)]
pub(crate) struct AdminGrantCreditsRequest {
    amount: i32,
    description: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/user/credits` — get current user's credit balance and history
pub(crate) async fn get_user_credits(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<UserCreditsResponse>>) {
    let state_guard = state.read().await;

    let user = match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "User not found")),
            );
        }
    };

    let transactions = state_guard
        .db
        .list_credit_transactions_for_user(auth_user.user_id)
        .await
        .unwrap_or_default();

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
pub(crate) async fn admin_grant_credits(
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

    let tx = CreditTransaction {
        id: Uuid::new_v4(),
        user_id: target_user.id,
        booking_id: None,
        amount: req.amount,
        transaction_type: CreditTransactionType::Grant,
        description: req.description.or(Some("Admin grant".to_string())),
        granted_by: Some(auth_user.user_id),
        created_at: Utc::now(),
    };
    if let Err(e) = state_guard.db.save_credit_transaction(&tx).await {
        tracing::warn!("Failed to save credit grant transaction: {e}");
    }

    (StatusCode::OK, Json(ApiResponse::success(())))
}

/// `POST /api/v1/admin/credits/refill-all` — refill all active users' credits (admin only)
pub(crate) async fn admin_refill_all_credits(
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
        if let Ok(()) = state_guard.db.save_user(&user).await {
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
                tracing::warn!("Failed to save refill transaction for user {}: {e}", user.id);
            }
            refilled += 1;
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(
            serde_json::json!({ "users_refilled": refilled }),
        )),
    )
}

/// `PUT /api/v1/admin/users/{id}/quota` — update a user's monthly credit quota (admin only)
pub(crate) async fn admin_update_user_quota(
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
