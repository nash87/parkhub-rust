//! Social / team handlers: absences, swap requests, waitlist, team view.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use uuid::Uuid;

use crate::audit::{AuditEntry, AuditEventType};
use parkhub_common::models::{
    Absence, AbsencePattern, AbsenceType, SwapRequest, SwapRequestStatus, WaitlistEntry,
};
use parkhub_common::{ApiResponse, Booking, BookingStatus, User};
use serde::{Deserialize, Serialize};

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// ABSENCES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
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
fn is_valid_date(s: &str) -> bool {
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
            let pattern = val.and_then(|json_str| serde_json::from_str::<AbsencePattern>(&json_str).ok());
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

// ═══════════════════════════════════════════════════════════════════════════════
// TEAM VIEW
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct TeamMemberStatus {
    user_id: Uuid,
    name: String,
    username: String,
    status: String,
    absence_type: Option<AbsenceType>,
}

/// `GET /api/v1/team/today` — return all users with their status today
#[utoipa::path(get, path = "/api/v1/team/today", tag = "Team",
    summary = "Team status today",
    description = "Returns all users with their status for today.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn team_today(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<TeamMemberStatus>>>) {
    let state_guard = state.read().await;

    let users = match state_guard.db.list_users().await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to load users")),
            );
        }
    };

    let today = Utc::now().format("%Y-%m-%d").to_string();

    let absences = state_guard
        .db
        .list_absences_team()
        .await
        .unwrap_or_default();

    let bookings = state_guard.db.list_bookings().await.unwrap_or_default();

    let mut result = Vec::new();
    for user in &users {
        if !user.is_active {
            continue;
        }

        // Check for absence today
        let user_absence = absences
            .iter()
            .find(|a| a.user_id == user.id && a.start_date <= today && a.end_date >= today);

        if let Some(absence) = user_absence {
            let status = match absence.absence_type {
                AbsenceType::Homeoffice => "homeoffice",
                AbsenceType::Vacation => "vacation",
                AbsenceType::Sick => "sick",
                AbsenceType::Training | AbsenceType::Other => "absent",
            };
            result.push(TeamMemberStatus {
                user_id: user.id,
                name: user.name.clone(),
                username: user.username.clone(),
                status: status.to_string(),
                absence_type: Some(absence.absence_type.clone()),
            });
            continue;
        }

        // Check for booking today (confirmed or active)
        let has_booking = bookings.iter().any(|b| {
            b.user_id == user.id
                && (b.status == BookingStatus::Confirmed || b.status == BookingStatus::Active)
                && b.start_time.format("%Y-%m-%d").to_string() <= today
                && b.end_time.format("%Y-%m-%d").to_string() >= today
        });

        let status = if has_booking { "parked" } else { "available" };
        result.push(TeamMemberStatus {
            user_id: user.id,
            name: user.name.clone(),
            username: user.username.clone(),
            status: status.to_string(),
            absence_type: None,
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// WAITLIST
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/waitlist` — list current user's waitlist entries
#[utoipa::path(get, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "List waitlist entries",
    description = "Returns waitlist entries for the authenticated user.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn list_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<WaitlistEntry>>> {
    let state_guard = state.read().await;
    match state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(entries) => Json(ApiResponse::success(entries)),
        Err(e) => {
            tracing::error!("Failed to list waitlist entries: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list waitlist entries",
            ))
        }
    }
}

/// Request body for joining the waitlist
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct JoinWaitlistRequest {
    lot_id: Uuid,
}

/// `POST /api/v1/waitlist` — join waitlist for a lot
#[utoipa::path(post, path = "/api/v1/waitlist", tag = "Waitlist",
    summary = "Join waitlist",
    description = "Adds the user to a lot waitlist.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn join_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<JoinWaitlistRequest>,
) -> (StatusCode, Json<ApiResponse<WaitlistEntry>>) {
    let state_guard = state.read().await;

    // Check waitlist_enabled setting
    let waitlist_enabled = read_admin_setting(&state_guard.db, "waitlist_enabled").await;
    if waitlist_enabled != "true" {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiResponse::error(
                "WAITLIST_DISABLED",
                "Waitlist is not enabled",
            )),
        );
    }

    // First-or-create: check if user already has a waitlist entry for this lot
    let existing = state_guard
        .db
        .list_waitlist_by_user(&auth_user.user_id.to_string())
        .await
        .unwrap_or_default();
    if let Some(entry) = existing.iter().find(|e| e.lot_id == req.lot_id) {
        return (StatusCode::OK, Json(ApiResponse::success(entry.clone())));
    }

    let entry = WaitlistEntry {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        lot_id: req.lot_id,
        created_at: Utc::now(),
        notified_at: None,
    };

    if let Err(e) = state_guard.db.save_waitlist_entry(&entry).await {
        tracing::error!("Failed to save waitlist entry: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to join waitlist",
            )),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(entry)))
}

/// `DELETE /api/v1/waitlist/{id}` — leave waitlist (verify ownership)
#[utoipa::path(delete, path = "/api/v1/waitlist/{id}", tag = "Waitlist",
    summary = "Leave waitlist",
    description = "Removes the user from a waitlist entry.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn leave_waitlist(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<()>>) {
    let state_guard = state.read().await;

    // Verify ownership
    match state_guard.db.get_waitlist_entry(&id).await {
        Ok(Some(entry)) => {
            if entry.user_id != auth_user.user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(ApiResponse::error("FORBIDDEN", "Access denied")),
                );
            }
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
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

    match state_guard.db.delete_waitlist_entry(&id).await {
        Ok(true) => (StatusCode::OK, Json(ApiResponse::success(()))),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Waitlist entry not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to delete waitlist entry: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to leave waitlist",
                )),
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SWAP REQUESTS
// ═══════════════════════════════════════════════════════════════════════════════

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
    target_booking_id: Uuid,
    message: Option<String>,
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
    action: String,
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

