//! Team view handlers: team status today, team member list.

use axum::{extract::State, http::StatusCode, Extension, Json};
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use parkhub_common::models::AbsenceType;
use parkhub_common::{ApiResponse, BookingStatus};

use super::{AuthUser, SharedState};

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

#[derive(Serialize)]
pub struct TeamMember {
    id: Uuid,
    name: String,
    username: String,
    role: String,
    is_active: bool,
}

/// `GET /api/v1/team` — list all team members (simplified view)
#[utoipa::path(get, path = "/api/v1/team", tag = "Team",
    summary = "List team members",
    description = "Returns all active team members.",
    security(("bearer_auth" = [])),
    responses((status = 200, description = "Success"))
)]
pub async fn team_list(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<TeamMember>>> {
    let state_guard = state.read().await;

    let users = state_guard.db.list_users().await.unwrap_or_default();
    let members: Vec<TeamMember> = users
        .into_iter()
        .filter(|u| u.is_active)
        .map(|u| TeamMember {
            id: u.id,
            name: u.name,
            username: u.username,
            role: format!("{:?}", u.role).to_lowercase(),
            is_active: u.is_active,
        })
        .collect();

    Json(ApiResponse::success(members))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkhub_common::models::AbsenceType;

    // ── TeamMemberStatus serialization ───────────────────────────────────────

    #[test]
    fn test_team_member_status_present_serialization() {
        let member = TeamMemberStatus {
            user_id: Uuid::nil(),
            name: "Alice Smith".to_string(),
            username: "alice".to_string(),
            status: "present".to_string(),
            absence_type: None,
        };
        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("Alice Smith"));
        assert!(json.contains("alice"));
        assert!(json.contains("present"));
        // absence_type should be serialised as null or missing when None
        // (serde default = include null)
        assert!(json.contains("absence_type"));
    }

    #[test]
    fn test_team_member_status_absent_homeoffice() {
        let member = TeamMemberStatus {
            user_id: Uuid::nil(),
            name: "Bob".to_string(),
            username: "bob".to_string(),
            status: "homeoffice".to_string(),
            absence_type: Some(AbsenceType::Homeoffice),
        };
        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains("homeoffice"));
        // AbsenceType::Homeoffice should be serialised as "homeoffice"
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["absence_type"], "homeoffice");
    }

    #[test]
    fn test_team_member_status_absent_vacation() {
        let member = TeamMemberStatus {
            user_id: Uuid::nil(),
            name: "Carol".to_string(),
            username: "carol".to_string(),
            status: "absent".to_string(),
            absence_type: Some(AbsenceType::Vacation),
        };
        let json = serde_json::to_string(&member).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["absence_type"], "vacation");
    }

    #[test]
    fn test_team_member_status_absent_sick() {
        let member = TeamMemberStatus {
            user_id: Uuid::nil(),
            name: "Dave".to_string(),
            username: "dave".to_string(),
            status: "absent".to_string(),
            absence_type: Some(AbsenceType::Sick),
        };
        let json = serde_json::to_string(&member).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["absence_type"], "sick");
    }

    #[test]
    fn test_team_member_uuid_is_serialized() {
        let id = Uuid::new_v4();
        let member = TeamMemberStatus {
            user_id: id,
            name: "Eve".to_string(),
            username: "eve".to_string(),
            status: "present".to_string(),
            absence_type: None,
        };
        let json = serde_json::to_string(&member).unwrap();
        assert!(json.contains(&id.to_string()));
    }
}
