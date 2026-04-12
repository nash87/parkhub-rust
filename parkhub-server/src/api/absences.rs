//! Absence handlers: CRUD for user absences, team absences, absence patterns.

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::{Absence, AbsencePattern, AbsenceType};
use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AbsenceQuery {
    #[serde(rename = "type")]
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/absences` — list current user's absences, optionally filtered by type
#[utoipa::path(get, path = "/api/v1/absences", tag = "Absences",
    summary = "List user absences",
    description = "Returns absences for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_absences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<AbsenceQuery>,
) -> (StatusCode, Json<ApiResponse<Vec<Absence>>>) {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_absences_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(absences) => {
            let filtered = match query.absence_type {
                Some(ref t) => absences
                    .into_iter()
                    .filter(|a| &a.absence_type == t)
                    .collect(),
                None => absences,
            };
            (StatusCode::OK, Json(ApiResponse::success(filtered)))
        }
        Err(e) => {
            tracing::error!("Failed to list absences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list absences",
                )),
            )
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateAbsenceRequest {
    absence_type: AbsenceType,
    start_date: String,
    end_date: String,
    note: Option<String>,
}

/// Validate a date string is YYYY-MM-DD format.
pub fn is_valid_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// `POST /api/v1/absences` — create an absence
#[utoipa::path(post, path = "/api/v1/absences", tag = "Absences",
    summary = "Create an absence",
    description = "Records a new absence for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn create_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<Absence>>) {
    if !is_valid_date(&req.start_date) || !is_valid_date(&req.end_date) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Dates must be in YYYY-MM-DD format",
            )),
        );
    }

    if req.start_date > req.end_date {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "start_date must not be after end_date",
            )),
        );
    }

    let absence = Absence {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        absence_type: req.absence_type,
        start_date: req.start_date,
        end_date: req.end_date,
        note: req.note,
        source: "manual".to_string(),
        created_at: Utc::now(),
    };

    let state_guard = state.read().await;
    match state_guard.db.save_absence(&absence).await {
        Ok(()) => (StatusCode::CREATED, Json(ApiResponse::success(absence))),
        Err(e) => {
            tracing::error!("Failed to save absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to create absence",
                )),
            )
        }
    }
}

/// `DELETE /api/v1/absences/{id}` — delete own absence
#[utoipa::path(delete, path = "/api/v1/absences/{id}", tag = "Absences",
    summary = "Delete an absence",
    description = "Removes an absence owned by the user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn delete_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership
    let absence = match state_guard.db.get_absence(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Absence not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching absence: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if absence.user_id != auth_user.user_id {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    match state_guard.db.delete_absence(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Absence not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to delete absence",
                )),
            )
        }
    }
}

/// `GET /api/v1/absences/team` — list all team absences
#[utoipa::path(
    get,
    path = "/api/v1/absences/team",
    tag = "Absences",
    summary = "List team absences",
    description = "List all team member absences visible to the current user.",
    security(("bearer_auth" = []))
)]
pub async fn list_team_absences(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<Absence>>>) {
    let state_guard = state.read().await;
    match state_guard.db.list_absences_team().await {
        Ok(absences) => (StatusCode::OK, Json(ApiResponse::success(absences))),
        Err(e) => {
            tracing::error!("Failed to list team absences: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list team absences",
                )),
            )
        }
    }
}

/// `GET /api/v1/absences/pattern` — get user's absence pattern
#[utoipa::path(
    get,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Get absence pattern",
    description = "Get the current user's recurring absence pattern.",
    security(("bearer_auth" = []))
)]
pub async fn get_absence_pattern(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Option<AbsencePattern>>>) {
    let state_guard = state.read().await;
    let key = format!("absence_pattern:{}", auth_user.user_id);
    match state_guard.db.get_setting(&key).await {
        Ok(val) => {
            let pattern =
                val.and_then(|json_str| serde_json::from_str::<AbsencePattern>(&json_str).ok());
            (StatusCode::OK, Json(ApiResponse::success(pattern)))
        }
        Err(e) => {
            tracing::error!("Failed to get absence pattern: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to get absence pattern",
                )),
            )
        }
    }
}

/// `POST /api/v1/absences/pattern` — save user's absence pattern
#[utoipa::path(
    post,
    path = "/api/v1/absences/pattern",
    tag = "Absences",
    summary = "Save absence pattern",
    description = "Save or update the current user's recurring absence pattern (e.g. homeoffice every Monday).",
    security(("bearer_auth" = []))
)]
pub async fn save_absence_pattern(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(pattern): Json<AbsencePattern>,
) -> (StatusCode, Json<ApiResponse<AbsencePattern>>) {
    let state_guard = state.read().await;
    let key = format!("absence_pattern:{}", auth_user.user_id);
    let json_str = match serde_json::to_string(&pattern) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to serialize absence pattern: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Serialization error")),
            );
        }
    };

    match state_guard.db.set_setting(&key, &json_str).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(pattern))),
        Err(e) => {
            tracing::error!("Failed to save absence pattern: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to save absence pattern",
                )),
            )
        }
    }
}

/// Request body for updating an absence
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateAbsenceRequest {
    pub absence_type: Option<AbsenceType>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub notes: Option<String>,
}

/// `PUT /api/v1/absences/{id}` — update an existing absence
#[utoipa::path(
    put,
    path = "/api/v1/absences/{id}",
    tag = "Absences",
    summary = "Update an absence",
    description = "Update absence_type, start_date, end_date, or notes. Only the owner or an admin may update.",
    security(("bearer_auth" = []))
)]
pub async fn update_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<UpdateAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<Absence>>) {
    let state_guard = state.read().await;

    let mut absence = match state_guard.db.get_absence(&id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Absence not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error fetching absence: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    // Check ownership or admin
    let Ok(Some(caller)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    else {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    };
    let is_admin = caller.role == UserRole::Admin || caller.role == UserRole::SuperAdmin;
    if absence.user_id != auth_user.user_id && !is_admin {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Access denied")),
        );
    }

    if let Some(absence_type) = req.absence_type {
        absence.absence_type = absence_type;
    }
    if let Some(start_date) = req.start_date {
        if !is_valid_date(&start_date) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "start_date must be in YYYY-MM-DD format",
                )),
            );
        }
        absence.start_date = start_date;
    }
    if let Some(end_date) = req.end_date {
        if !is_valid_date(&end_date) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID_INPUT",
                    "end_date must be in YYYY-MM-DD format",
                )),
            );
        }
        absence.end_date = end_date;
    }
    if let Some(notes) = req.notes {
        absence.note = Some(notes);
    }

    if absence.start_date > absence.end_date {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "start_date must not be after end_date",
            )),
        );
    }

    match state_guard.db.save_absence(&absence).await {
        Ok(()) => (StatusCode::OK, Json(ApiResponse::success(absence))),
        Err(e) => {
            tracing::error!("Failed to update absence: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to update absence",
                )),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_date_correct() {
        assert!(is_valid_date("2026-03-20"));
        assert!(is_valid_date("2000-01-01"));
        assert!(is_valid_date("2026-12-31"));
    }

    #[test]
    fn test_is_valid_date_invalid() {
        assert!(!is_valid_date("2026-13-01"));
        assert!(!is_valid_date("2026-02-30"));
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date(""));
        assert!(!is_valid_date("20260320"));
        assert!(!is_valid_date("2026/03/20"));
    }

    #[test]
    fn test_is_valid_date_leap_year() {
        assert!(is_valid_date("2024-02-29"));
        assert!(!is_valid_date("2025-02-29"));
    }

    #[test]
    fn test_create_absence_request() {
        let json = r#"{"absence_type":"homeoffice","start_date":"2026-04-01","end_date":"2026-04-01","note":"WFH"}"#;
        let req: CreateAbsenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.start_date, "2026-04-01");
        assert_eq!(req.end_date, "2026-04-01");
        assert_eq!(req.note.as_deref(), Some("WFH"));
    }

    #[test]
    fn test_create_absence_request_no_note() {
        let json =
            r#"{"absence_type":"vacation","start_date":"2026-04-01","end_date":"2026-04-05"}"#;
        let req: CreateAbsenceRequest = serde_json::from_str(json).unwrap();
        assert!(req.note.is_none());
    }
}
