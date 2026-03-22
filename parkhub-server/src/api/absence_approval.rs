//! Absence Approval Workflow handlers.
//!
//! Submit absence requests that require admin approval before becoming effective.
//!
//! - `POST /api/v1/absences/requests` — submit absence request (pending state)
//! - `GET /api/v1/admin/absences/pending` — list pending approvals
//! - `PUT /api/v1/admin/absences/{id}/approve` — approve with optional comment
//! - `PUT /api/v1/admin/absences/{id}/reject` — reject with reason
//! - `GET /api/v1/absences/my` — user's absence history with approval status

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Approval state for an absence request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

/// An absence request with approval workflow
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AbsenceRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub absence_type: String,
    pub start_date: String,
    pub end_date: String,
    pub reason: String,
    pub status: ApprovalStatus,
    pub reviewer_id: Option<Uuid>,
    pub reviewer_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// Request body for submitting an absence request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SubmitAbsenceRequest {
    pub absence_type: String,
    pub start_date: String,
    pub end_date: String,
    pub reason: String,
}

/// Request body for approving an absence
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ApproveAbsenceRequest {
    pub comment: Option<String>,
}

/// Request body for rejecting an absence
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RejectAbsenceRequest {
    pub reason: String,
}

/// Notification generated on status change
#[derive(Debug, Clone, Serialize)]
pub struct AbsenceNotification {
    pub user_id: Uuid,
    pub message: String,
    pub status: ApprovalStatus,
    pub absence_request_id: Uuid,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate a date string is YYYY-MM-DD format.
fn is_valid_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// Valid absence types
const VALID_ABSENCE_TYPES: &[&str] = &[
    "vacation",
    "sick",
    "homeoffice",
    "business_trip",
    "personal",
    "other",
];

/// Check if absence type is valid
fn is_valid_absence_type(t: &str) -> bool {
    VALID_ABSENCE_TYPES.contains(&t)
}

/// Build a notification for status change
fn build_notification(req: &AbsenceRequest) -> AbsenceNotification {
    let message = match req.status {
        ApprovalStatus::Approved => format!(
            "Your absence request ({} – {}) has been approved.",
            req.start_date, req.end_date
        ),
        ApprovalStatus::Rejected => format!(
            "Your absence request ({} – {}) has been rejected. Reason: {}",
            req.start_date,
            req.end_date,
            req.reviewer_comment.as_deref().unwrap_or("No reason given")
        ),
        ApprovalStatus::Pending => format!(
            "Your absence request ({} – {}) is pending review.",
            req.start_date, req.end_date
        ),
    };
    AbsenceNotification {
        user_id: req.user_id,
        message,
        status: req.status.clone(),
        absence_request_id: req.id,
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `POST /api/v1/absences/requests` — submit an absence request (pending state)
#[utoipa::path(post, path = "/api/v1/absences/requests", tag = "Absence Approval",
    summary = "Submit absence request",
    description = "Submit a new absence request that requires admin approval.",
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Request submitted"),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn submit_absence_request(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<SubmitAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<AbsenceRequest>>) {
    // Validate dates
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

    if !is_valid_absence_type(&req.absence_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Invalid absence type",
            )),
        );
    }

    if req.reason.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Reason is required",
            )),
        );
    }

    let state_guard = state.read().await;

    // Get user name
    let user_name = if let Ok(Some(user)) = state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        user.name
    } else {
        "Unknown".to_string()
    };

    let absence_request = AbsenceRequest {
        id: Uuid::new_v4(),
        user_id: auth_user.user_id,
        user_name,
        absence_type: req.absence_type,
        start_date: req.start_date,
        end_date: req.end_date,
        reason: req.reason,
        status: ApprovalStatus::Pending,
        reviewer_id: None,
        reviewer_comment: None,
        created_at: Utc::now(),
        reviewed_at: None,
    };

    // Persist as a setting (JSON blob keyed by request ID)
    let key = format!("absence_request:{}", absence_request.id);
    let json_str = match serde_json::to_string(&absence_request) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to serialize absence request: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Serialization error")),
            );
        }
    };

    if let Err(e) = state_guard.db.set_setting(&key, &json_str).await {
        tracing::error!("Failed to save absence request: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save absence request",
            )),
        );
    }

    // Track in the user's request list
    let list_key = format!("absence_requests_user:{}", auth_user.user_id);
    let mut ids = load_id_list(&state_guard.db, &list_key).await;
    ids.push(absence_request.id);
    save_id_list(&state_guard.db, &list_key, &ids).await;

    // Track in the global pending list
    let mut pending = load_id_list(&state_guard.db, "absence_requests_pending").await;
    pending.push(absence_request.id);
    save_id_list(&state_guard.db, "absence_requests_pending", &pending).await;

    (
        StatusCode::CREATED,
        Json(ApiResponse::success(absence_request)),
    )
}

/// `GET /api/v1/admin/absences/pending` — list pending absence requests
#[utoipa::path(get, path = "/api/v1/admin/absences/pending", tag = "Absence Approval",
    summary = "List pending absence requests",
    description = "Admin endpoint to list all pending absence requests.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of pending requests"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn list_pending_absences(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<AbsenceRequest>>>) {
    let state_guard = state.read().await;

    // Verify admin
    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let pending_ids = load_id_list(&state_guard.db, "absence_requests_pending").await;
    let mut requests = Vec::new();

    for id in &pending_ids {
        let key = format!("absence_request:{id}");
        if let Ok(Some(json_str)) = state_guard.db.get_setting(&key).await {
            if let Ok(req) = serde_json::from_str::<AbsenceRequest>(&json_str) {
                if req.status == ApprovalStatus::Pending {
                    requests.push(req);
                }
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(requests)))
}

/// `PUT /api/v1/admin/absences/{id}/approve` — approve an absence request
#[utoipa::path(put, path = "/api/v1/admin/absences/{id}/approve", tag = "Absence Approval",
    summary = "Approve absence request",
    description = "Approve a pending absence request with an optional comment.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Request approved"),
        (status = 404, description = "Request not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn approve_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<ApproveAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<AbsenceRequest>>) {
    let state_guard = state.read().await;

    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let key = format!("absence_request:{id}");
    let json_str = match state_guard.db.get_setting(&key).await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Absence request not found")),
            )
        }
    };

    let mut request: AbsenceRequest = match serde_json::from_str(&json_str) {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Corrupt request data")),
            )
        }
    };

    if request.status != ApprovalStatus::Pending {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_STATE",
                "Request is not pending",
            )),
        );
    }

    request.status = ApprovalStatus::Approved;
    request.reviewer_id = Some(auth_user.user_id);
    request.reviewer_comment = body.comment;
    request.reviewed_at = Some(Utc::now());

    // Save updated request
    let updated_json = serde_json::to_string(&request).unwrap_or_default();
    let _ = state_guard.db.set_setting(&key, &updated_json).await;

    // Remove from pending list
    remove_from_pending(&state_guard.db, &id).await;

    // Generate notification
    let notification = build_notification(&request);
    tracing::info!(
        "Absence approval notification for user {}: {}",
        notification.user_id,
        notification.message
    );

    (StatusCode::OK, Json(ApiResponse::success(request)))
}

/// `PUT /api/v1/admin/absences/{id}/reject` — reject an absence request
#[utoipa::path(put, path = "/api/v1/admin/absences/{id}/reject", tag = "Absence Approval",
    summary = "Reject absence request",
    description = "Reject a pending absence request with a reason.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Request rejected"),
        (status = 404, description = "Request not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn reject_absence(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<RejectAbsenceRequest>,
) -> (StatusCode, Json<ApiResponse<AbsenceRequest>>) {
    let state_guard = state.read().await;

    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    if body.reason.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Rejection reason is required",
            )),
        );
    }

    let key = format!("absence_request:{id}");
    let json_str = match state_guard.db.get_setting(&key).await {
        Ok(Some(s)) => s,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Absence request not found")),
            )
        }
    };

    let mut request: AbsenceRequest = match serde_json::from_str(&json_str) {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Corrupt request data")),
            )
        }
    };

    if request.status != ApprovalStatus::Pending {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_STATE",
                "Request is not pending",
            )),
        );
    }

    request.status = ApprovalStatus::Rejected;
    request.reviewer_id = Some(auth_user.user_id);
    request.reviewer_comment = Some(body.reason);
    request.reviewed_at = Some(Utc::now());

    // Save updated request
    let updated_json = serde_json::to_string(&request).unwrap_or_default();
    let _ = state_guard.db.set_setting(&key, &updated_json).await;

    // Remove from pending list
    remove_from_pending(&state_guard.db, &id).await;

    // Generate notification
    let notification = build_notification(&request);
    tracing::info!(
        "Absence rejection notification for user {}: {}",
        notification.user_id,
        notification.message
    );

    (StatusCode::OK, Json(ApiResponse::success(request)))
}

/// `GET /api/v1/absences/my` — user's absence request history with status
#[utoipa::path(get, path = "/api/v1/absences/my", tag = "Absence Approval",
    summary = "My absence requests",
    description = "List the current user's absence requests with approval status.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of user's absence requests"),
    )
)]
pub async fn my_absence_requests(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<AbsenceRequest>>> {
    let state_guard = state.read().await;
    let list_key = format!("absence_requests_user:{}", auth_user.user_id);
    let ids = load_id_list(&state_guard.db, &list_key).await;

    let mut requests = Vec::new();
    for id in &ids {
        let key = format!("absence_request:{id}");
        if let Ok(Some(json_str)) = state_guard.db.get_setting(&key).await {
            if let Ok(req) = serde_json::from_str::<AbsenceRequest>(&json_str) {
                requests.push(req);
            }
        }
    }

    // Sort newest first
    requests.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Json(ApiResponse::success(requests))
}

// ═══════════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Check if user is admin
async fn is_admin(state: &crate::AppState, auth_user: &AuthUser) -> bool {
    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u.role == UserRole::Admin || u.role == UserRole::SuperAdmin,
        _ => false,
    }
}

/// Load a list of UUIDs from a setting key
async fn load_id_list(db: &crate::db::Database, key: &str) -> Vec<Uuid> {
    match db.get_setting(key).await {
        Ok(Some(json_str)) => serde_json::from_str(&json_str).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Save a list of UUIDs to a setting key
async fn save_id_list(db: &crate::db::Database, key: &str, ids: &[Uuid]) {
    if let Ok(json_str) = serde_json::to_string(ids) {
        let _ = db.set_setting(key, &json_str).await;
    }
}

/// Remove an ID from the pending list
async fn remove_from_pending(db: &crate::db::Database, id: &Uuid) {
    let mut pending = load_id_list(db, "absence_requests_pending").await;
    pending.retain(|pid| pid != id);
    save_id_list(db, "absence_requests_pending", &pending).await;
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_status_serialize() {
        assert_eq!(
            serde_json::to_string(&ApprovalStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalStatus::Approved).unwrap(),
            "\"approved\""
        );
        assert_eq!(
            serde_json::to_string(&ApprovalStatus::Rejected).unwrap(),
            "\"rejected\""
        );
    }

    #[test]
    fn test_approval_status_deserialize() {
        let p: ApprovalStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(p, ApprovalStatus::Pending);
        let a: ApprovalStatus = serde_json::from_str("\"approved\"").unwrap();
        assert_eq!(a, ApprovalStatus::Approved);
        let r: ApprovalStatus = serde_json::from_str("\"rejected\"").unwrap();
        assert_eq!(r, ApprovalStatus::Rejected);
    }

    #[test]
    fn test_approval_status_display() {
        assert_eq!(ApprovalStatus::Pending.to_string(), "pending");
        assert_eq!(ApprovalStatus::Approved.to_string(), "approved");
        assert_eq!(ApprovalStatus::Rejected.to_string(), "rejected");
    }

    #[test]
    fn test_approval_status_equality() {
        assert_eq!(ApprovalStatus::Pending, ApprovalStatus::Pending);
        assert_ne!(ApprovalStatus::Pending, ApprovalStatus::Approved);
        assert_ne!(ApprovalStatus::Approved, ApprovalStatus::Rejected);
    }

    #[test]
    fn test_absence_request_serialize() {
        let req = AbsenceRequest {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            user_name: "Alice".to_string(),
            absence_type: "vacation".to_string(),
            start_date: "2026-04-01".to_string(),
            end_date: "2026-04-05".to_string(),
            reason: "Family trip".to_string(),
            status: ApprovalStatus::Pending,
            reviewer_id: None,
            reviewer_comment: None,
            created_at: Utc::now(),
            reviewed_at: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"user_name\":\"Alice\""));
        assert!(json.contains("\"absence_type\":\"vacation\""));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"reason\":\"Family trip\""));
    }

    #[test]
    fn test_absence_request_roundtrip() {
        let req = AbsenceRequest {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Bob".to_string(),
            absence_type: "sick".to_string(),
            start_date: "2026-03-20".to_string(),
            end_date: "2026-03-21".to_string(),
            reason: "Flu".to_string(),
            status: ApprovalStatus::Approved,
            reviewer_id: Some(Uuid::new_v4()),
            reviewer_comment: Some("Get well soon".to_string()),
            created_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: AbsenceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, req.id);
        assert_eq!(parsed.user_name, "Bob");
        assert_eq!(parsed.status, ApprovalStatus::Approved);
        assert_eq!(parsed.reviewer_comment.as_deref(), Some("Get well soon"));
    }

    #[test]
    fn test_submit_request_deserialize() {
        let json = r#"{"absence_type":"vacation","start_date":"2026-04-01","end_date":"2026-04-05","reason":"Family trip"}"#;
        let req: SubmitAbsenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.absence_type, "vacation");
        assert_eq!(req.start_date, "2026-04-01");
        assert_eq!(req.end_date, "2026-04-05");
        assert_eq!(req.reason, "Family trip");
    }

    #[test]
    fn test_approve_request_deserialize() {
        let json = r#"{"comment":"Enjoy your time off"}"#;
        let req: ApproveAbsenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.comment.as_deref(), Some("Enjoy your time off"));

        let json2 = r#"{}"#;
        let req2: ApproveAbsenceRequest = serde_json::from_str(json2).unwrap();
        assert!(req2.comment.is_none());
    }

    #[test]
    fn test_reject_request_deserialize() {
        let json = r#"{"reason":"Staffing conflicts"}"#;
        let req: RejectAbsenceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.reason, "Staffing conflicts");
    }

    #[test]
    fn test_is_valid_date() {
        assert!(is_valid_date("2026-04-01"));
        assert!(is_valid_date("2024-02-29")); // leap year
        assert!(!is_valid_date("2025-02-29"));
        assert!(!is_valid_date("not-a-date"));
        assert!(!is_valid_date(""));
        assert!(!is_valid_date("2026/04/01"));
    }

    #[test]
    fn test_is_valid_absence_type() {
        assert!(is_valid_absence_type("vacation"));
        assert!(is_valid_absence_type("sick"));
        assert!(is_valid_absence_type("homeoffice"));
        assert!(is_valid_absence_type("business_trip"));
        assert!(is_valid_absence_type("personal"));
        assert!(is_valid_absence_type("other"));
        assert!(!is_valid_absence_type("invalid"));
        assert!(!is_valid_absence_type(""));
    }

    #[test]
    fn test_build_notification_approved() {
        let req = AbsenceRequest {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Alice".to_string(),
            absence_type: "vacation".to_string(),
            start_date: "2026-04-01".to_string(),
            end_date: "2026-04-05".to_string(),
            reason: "Trip".to_string(),
            status: ApprovalStatus::Approved,
            reviewer_id: Some(Uuid::new_v4()),
            reviewer_comment: Some("Approved".to_string()),
            created_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        };
        let notification = build_notification(&req);
        assert_eq!(notification.user_id, req.user_id);
        assert!(notification.message.contains("approved"));
        assert_eq!(notification.status, ApprovalStatus::Approved);
    }

    #[test]
    fn test_build_notification_rejected() {
        let req = AbsenceRequest {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Bob".to_string(),
            absence_type: "vacation".to_string(),
            start_date: "2026-04-01".to_string(),
            end_date: "2026-04-05".to_string(),
            reason: "Trip".to_string(),
            status: ApprovalStatus::Rejected,
            reviewer_id: Some(Uuid::new_v4()),
            reviewer_comment: Some("Staffing conflicts".to_string()),
            created_at: Utc::now(),
            reviewed_at: Some(Utc::now()),
        };
        let notification = build_notification(&req);
        assert!(notification.message.contains("rejected"));
        assert!(notification.message.contains("Staffing conflicts"));
    }

    #[test]
    fn test_build_notification_pending() {
        let req = AbsenceRequest {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            user_name: "Carol".to_string(),
            absence_type: "sick".to_string(),
            start_date: "2026-03-20".to_string(),
            end_date: "2026-03-21".to_string(),
            reason: "Flu".to_string(),
            status: ApprovalStatus::Pending,
            reviewer_id: None,
            reviewer_comment: None,
            created_at: Utc::now(),
            reviewed_at: None,
        };
        let notification = build_notification(&req);
        assert!(notification.message.contains("pending review"));
    }
}
