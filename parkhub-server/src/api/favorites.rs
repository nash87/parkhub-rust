//! Favorites handlers: users can pin preferred parking slots.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use crate::db::Favorite;

use super::{AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Request DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AddFavoriteRequest {
    pub slot_id: Uuid,
    pub lot_id: Uuid,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/user/favorites` — list the authenticated user's favorite slots
pub(crate) async fn list_favorites(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<Favorite>>> {
    let state = state.read().await;

    match state
        .db
        .list_favorites_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(favs) => Json(ApiResponse::success(favs)),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list favorites",
            ))
        }
    }
}

/// `POST /api/v1/user/favorites` — add a favorite slot
pub(crate) async fn add_favorite(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AddFavoriteRequest>,
) -> (StatusCode, Json<ApiResponse<Favorite>>) {
    let state_guard = state.read().await;

    // Verify slot exists
    match state_guard
        .db
        .get_parking_slot(&req.slot_id.to_string())
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Parking slot not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    }

    let fav = Favorite {
        user_id: auth_user.user_id,
        slot_id: req.slot_id,
        lot_id: req.lot_id,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_favorite(&fav).await {
        tracing::error!("Failed to save favorite: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to add favorite")),
        );
    }

    tracing::info!(
        "User {} favorited slot {} in lot {}",
        fav.user_id,
        fav.slot_id,
        fav.lot_id
    );

    (StatusCode::CREATED, Json(ApiResponse::success(fav)))
}

/// `DELETE /api/v1/user/favorites/{slot_id}` — remove a favorite slot
pub(crate) async fn remove_favorite(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(slot_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    match state_guard
        .db
        .delete_favorite(&auth_user.user_id.to_string(), &slot_id)
        .await
    {
        Ok(true) => {
            tracing::info!(
                "User {} removed favorite slot {}",
                auth_user.user_id,
                slot_id
            );
            (StatusCode::OK, Json(ApiResponse::success(())))
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Favorite not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete favorite: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to remove favorite",
                )),
            )
        }
    }
}
