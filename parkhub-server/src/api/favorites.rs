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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddFavoriteRequest {
    /// Parking slot ID to favorite
    pub slot_id: Uuid,
    /// Parking lot ID the slot belongs to
    pub lot_id: Uuid,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/user/favorites` — list the authenticated user's favorite slots
#[utoipa::path(
    get,
    path = "/api/v1/user/favorites",
    tag = "Favorites",
    summary = "List favorite slots",
    description = "Returns the authenticated user's pinned/favorite parking slots.",
    responses(
        (status = 200, description = "List of favorites"),
    )
)]
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
#[utoipa::path(
    post,
    path = "/api/v1/user/favorites",
    tag = "Favorites",
    summary = "Add a favorite slot",
    description = "Pin a parking slot as a favorite for quick access.",
    request_body = AddFavoriteRequest,
    responses(
        (status = 201, description = "Favorite added"),
        (status = 404, description = "Parking slot not found"),
    )
)]
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
#[utoipa::path(
    delete,
    path = "/api/v1/user/favorites/{slot_id}",
    tag = "Favorites",
    summary = "Remove a favorite slot",
    description = "Unpin a parking slot from the user's favorites.",
    params(("slot_id" = String, Path, description = "Parking slot ID")),
    responses(
        (status = 200, description = "Favorite removed"),
        (status = 404, description = "Favorite not found"),
    )
)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_favorite_request_deserialize() {
        let json = r#"{"slot_id":"550e8400-e29b-41d4-a716-446655440000","lot_id":"660e8400-e29b-41d4-a716-446655440001"}"#;
        let req: AddFavoriteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            req.slot_id.to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
        assert_eq!(
            req.lot_id.to_string(),
            "660e8400-e29b-41d4-a716-446655440001"
        );
    }

    #[test]
    fn test_add_favorite_request_missing_field() {
        let json = r#"{"slot_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let result: Result<AddFavoriteRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_favorite_request_invalid_uuid() {
        let json = r#"{"slot_id":"not-a-uuid","lot_id":"also-bad"}"#;
        let result: Result<AddFavoriteRequest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_favorite_serialize_roundtrip() {
        let fav = Favorite {
            user_id: Uuid::new_v4(),
            slot_id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&fav).unwrap();
        let deserialized: Favorite = serde_json::from_str(&json).unwrap();
        assert_eq!(fav.user_id, deserialized.user_id);
        assert_eq!(fav.slot_id, deserialized.slot_id);
        assert_eq!(fav.lot_id, deserialized.lot_id);
    }
}
