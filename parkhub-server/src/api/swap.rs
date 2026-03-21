//! Swap request handlers: list, create, accept/decline.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::{SwapRequest, SwapRequestStatus};
use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState};

/// `GET /api/v1/swap-requests` — list user's swap requests (as requester or target)
#[utoipa::path(
    get,
    path = "/api/v1/swap-requests",
    tag = "Bookings",
    summary = "List swap requests",
    description = "List the current user's swap requests (as requester or target).",
    security(("bearer_auth" = []))
)]
pub async fn list_swap_requests(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<SwapRequest>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_swap_requests_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(requests) => Json(ApiResponse::success(requests)),
        Err(e) => {
            tracing::error!("Failed to list swap requests: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list swap requests",
            ))
        }
    }
}

/// Request body for creating a swap request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateSwapRequestBody {
    pub target_booking_id: Uuid,
    pub message: Option<String>,
}

/// `POST /api/v1/bookings/{id}/swap-request` — create a swap request
#[utoipa::path(
    post,
    path = "/api/v1/bookings/{id}/swap-request",
    tag = "Bookings",
    summary = "Create swap request",
    description = "Create a parking slot swap request for a booking.",
    security(("bearer_auth" = []))
)]
pub async fn create_swap_request(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(booking_id): Path<String>,
    Json(req): Json<CreateSwapRequestBody>,
) -> (StatusCode, Json<ApiResponse<SwapRequest>>) {
    let state_guard = state.read().await;

    // Get requester's booking
    let requester_booking = match state_guard.db.get_booking(&booking_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Booking not found")),
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

    // Verify ownership of requester booking
    if requester_booking.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "You can only create swap requests for your own bookings",
            )),
        );
    }

    // Get target booking
    let target_booking = match state_guard
        .db
        .get_booking(&req.target_booking_id.to_string())
        .await
    {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Target booking not found")),
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

    // Validate: different users
    if requester_booking.user_id == target_booking.user_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_SWAP",
                "Cannot swap with your own booking",
            )),
        );
    }

    // Validate: same lot
    if requester_booking.lot_id != target_booking.lot_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_SWAP",
                "Bookings must be in the same lot",
            )),
        );
    }

    let swap_request = SwapRequest {
        id: Uuid::new_v4(),
        requester_booking_id: requester_booking.id,
        target_booking_id: target_booking.id,
        requester_id: auth_user.user_id,
        target_id: target_booking.user_id,
        status: SwapRequestStatus::Pending,
        message: req.message,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_swap_request(&swap_request).await {
        tracing::error!("Failed to save swap request: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to create swap request",
            )),
        );
    }

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(swap_request)),
    )
}

/// Request body for accepting/declining a swap request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateSwapRequestBody {
    pub action: String,
}

/// `PUT /api/v1/swap-requests/{id}` — accept or decline a swap request
#[utoipa::path(
    put,
    path = "/api/v1/swap-requests/{id}",
    tag = "Bookings",
    summary = "Update swap request",
    description = "Accept or decline a swap request.",
    security(("bearer_auth" = []))
)]
pub async fn update_swap_request(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSwapRequestBody>,
) -> (StatusCode, Json<ApiResponse<SwapRequest>>) {
    // Use write lock for atomic swap if accepting
    let state_guard = state.write().await;

    let mut swap = match state_guard.db.get_swap_request(&id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Swap request not found")),
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

    // Only the target user can accept/decline
    if swap.target_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(
                "FORBIDDEN",
                "Only the target user can respond to this swap request",
            )),
        );
    }

    if swap.status != SwapRequestStatus::Pending {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "ALREADY_RESOLVED",
                "This swap request has already been resolved",
            )),
        );
    }

    match req.action.as_str() {
        "accept" => {
            // Get both bookings
            let Ok(Some(mut requester_booking)) = state_guard
                .db
                .get_booking(&swap.requester_booking_id.to_string())
                .await
            else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Requester booking not found",
                    )),
                );
            };

            let Ok(Some(mut target_booking)) = state_guard
                .db
                .get_booking(&swap.target_booking_id.to_string())
                .await
            else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "SERVER_ERROR",
                        "Target booking not found",
                    )),
                );
            };

            // Swap slot_ids between the two bookings
            std::mem::swap(&mut requester_booking.slot_id, &mut target_booking.slot_id);
            std::mem::swap(
                &mut requester_booking.slot_number,
                &mut target_booking.slot_number,
            );
            std::mem::swap(
                &mut requester_booking.floor_name,
                &mut target_booking.floor_name,
            );
            let now = Utc::now();
            requester_booking.updated_at = now;
            target_booking.updated_at = now;

            if let Err(e) = state_guard.db.save_booking(&requester_booking).await {
                tracing::error!("Failed to save requester booking during swap: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to perform swap")),
                );
            }
            if let Err(e) = state_guard.db.save_booking(&target_booking).await {
                tracing::error!("Failed to save target booking during swap: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to perform swap")),
                );
            }

            swap.status = SwapRequestStatus::Accepted;
        }
        "decline" => {
            swap.status = SwapRequestStatus::Declined;
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_ACTION",
                    "Action must be 'accept' or 'decline'",
                )),
            );
        }
    }

    if let Err(e) = state_guard.db.save_swap_request(&swap).await {
        tracing::error!("Failed to update swap request: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to update swap request",
            )),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(swap)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_swap_request_body() {
        let json = r#"{"target_booking_id":"550e8400-e29b-41d4-a716-446655440000","message":"Please swap?"}"#;
        let req: CreateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.target_booking_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(req.message.as_deref(), Some("Please swap?"));
    }

    #[test]
    fn test_create_swap_request_body_no_message() {
        let json = r#"{"target_booking_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: CreateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert!(req.message.is_none());
    }

    #[test]
    fn test_update_swap_request_body_accept() {
        let json = r#"{"action":"accept"}"#;
        let req: UpdateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "accept");
    }

    #[test]
    fn test_update_swap_request_body_decline() {
        let json = r#"{"action":"decline"}"#;
        let req: UpdateSwapRequestBody = serde_json::from_str(json).unwrap();
        assert_eq!(req.action, "decline");
    }
}
