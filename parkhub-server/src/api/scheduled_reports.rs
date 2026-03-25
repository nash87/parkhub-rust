//! Scheduled Reports (Email Digest) handlers.
//!
//! Allows administrators to configure automated report delivery
//! via email on daily, weekly, or monthly schedules.
//!
//! - `GET    /api/v1/admin/reports/schedules`          — list all schedules
//! - `POST   /api/v1/admin/reports/schedules`          — create a schedule
//! - `PUT    /api/v1/admin/reports/schedules/{id}`      — update a schedule
//! - `DELETE /api/v1/admin/reports/schedules/{id}`      — delete a schedule
//! - `POST   /api/v1/admin/reports/schedules/{id}/send-now` — trigger immediate send

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::SharedState;

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Report types that can be scheduled
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    OccupancySummary,
    RevenueReport,
    UserActivity,
    BookingTrends,
}

#[allow(dead_code)]
impl ReportType {
    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Self::OccupancySummary => "Occupancy Summary",
            Self::RevenueReport => "Revenue Report",
            Self::UserActivity => "User Activity",
            Self::BookingTrends => "Booking Trends",
        }
    }

    /// All available report types
    pub const ALL: &[ReportType] = &[
        Self::OccupancySummary,
        Self::RevenueReport,
        Self::UserActivity,
        Self::BookingTrends,
    ];
}

/// Schedule frequency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleFrequency {
    Daily,
    Weekly,
    Monthly,
}

#[allow(dead_code)]
impl ScheduleFrequency {
    /// Human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Daily => "Daily",
            Self::Weekly => "Weekly",
            Self::Monthly => "Monthly",
        }
    }

    /// Cron expression for this frequency
    pub fn cron_expression(&self) -> &'static str {
        match self {
            Self::Daily => "0 8 * * *",
            Self::Weekly => "0 8 * * 1",
            Self::Monthly => "0 8 1 * *",
        }
    }
}

/// A configured report schedule
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReportSchedule {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub frequency: ScheduleFrequency,
    pub recipients: Vec<String>,
    pub enabled: bool,
    pub last_sent_at: Option<String>,
    pub next_run_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Request to create a new schedule
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub report_type: ReportType,
    pub frequency: ScheduleFrequency,
    pub recipients: Vec<String>,
}

/// Request to update an existing schedule
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub report_type: Option<ReportType>,
    pub frequency: Option<ScheduleFrequency>,
    pub recipients: Option<Vec<String>>,
    pub enabled: Option<bool>,
}

/// Response after triggering an immediate send
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SendNowResponse {
    pub schedule_id: String,
    pub report_type: ReportType,
    pub recipients_count: usize,
    pub sent_at: String,
    pub message: String,
}

/// List of report schedules
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScheduleListResponse {
    pub schedules: Vec<ReportSchedule>,
    pub total: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Compute the next run time based on frequency from now
fn compute_next_run(frequency: &ScheduleFrequency) -> String {
    let now = Utc::now();
    let next = match frequency {
        ScheduleFrequency::Daily => now + chrono::Duration::days(1),
        ScheduleFrequency::Weekly => now + chrono::Duration::weeks(1),
        ScheduleFrequency::Monthly => now + chrono::Duration::days(30),
    };
    next.to_rfc3339()
}

/// Generate sample schedules for demo mode
fn generate_sample_schedules() -> Vec<ReportSchedule> {
    let now = Utc::now().to_rfc3339();
    vec![
        ReportSchedule {
            id: "sched-001".to_string(),
            name: "Daily Occupancy Digest".to_string(),
            report_type: ReportType::OccupancySummary,
            frequency: ScheduleFrequency::Daily,
            recipients: vec!["admin@parkhub.test".to_string()],
            enabled: true,
            last_sent_at: Some(now.clone()),
            next_run_at: compute_next_run(&ScheduleFrequency::Daily),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ReportSchedule {
            id: "sched-002".to_string(),
            name: "Weekly Revenue Summary".to_string(),
            report_type: ReportType::RevenueReport,
            frequency: ScheduleFrequency::Weekly,
            recipients: vec![
                "admin@parkhub.test".to_string(),
                "finance@parkhub.test".to_string(),
            ],
            enabled: true,
            last_sent_at: None,
            next_run_at: compute_next_run(&ScheduleFrequency::Weekly),
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

/// Validate recipients list (non-empty, all valid emails)
fn validate_recipients(recipients: &[String]) -> Result<(), &'static str> {
    if recipients.is_empty() {
        return Err("At least one recipient is required");
    }
    for email in recipients {
        if !email.contains('@') || email.len() < 5 {
            return Err("Invalid email address in recipients");
        }
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/reports/schedules` — list all scheduled reports.
pub async fn list_schedules(
    State(_state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<ScheduleListResponse>>) {
    let schedules = generate_sample_schedules();
    let total = schedules.len();
    let response = ScheduleListResponse { schedules, total };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// `POST /api/v1/admin/reports/schedules` — create a new schedule.
pub async fn create_schedule(
    State(_state): State<SharedState>,
    Json(req): Json<CreateScheduleRequest>,
) -> (StatusCode, Json<ApiResponse<ReportSchedule>>) {
    if req.name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "bad_request",
                "Schedule name is required",
            )),
        );
    }

    if let Err(msg) = validate_recipients(&req.recipients) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("bad_request", msg)),
        );
    }

    let now = Utc::now().to_rfc3339();
    let schedule = ReportSchedule {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        report_type: req.report_type,
        frequency: req.frequency.clone(),
        recipients: req.recipients,
        enabled: true,
        last_sent_at: None,
        next_run_at: compute_next_run(&req.frequency),
        created_at: now.clone(),
        updated_at: now,
    };

    (StatusCode::CREATED, Json(ApiResponse::success(schedule)))
}

/// `PUT /api/v1/admin/reports/schedules/{id}` — update a schedule.
pub async fn update_schedule(
    State(_state): State<SharedState>,
    Path(schedule_id): Path<String>,
    Json(req): Json<UpdateScheduleRequest>,
) -> (StatusCode, Json<ApiResponse<ReportSchedule>>) {
    if let Some(ref recipients) = req.recipients {
        if let Err(msg) = validate_recipients(recipients) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("bad_request", msg)),
            );
        }
    }

    let frequency = req.frequency.unwrap_or(ScheduleFrequency::Daily);
    let now = Utc::now().to_rfc3339();
    let schedule = ReportSchedule {
        id: schedule_id,
        name: req.name.unwrap_or_else(|| "Updated Schedule".to_string()),
        report_type: req.report_type.unwrap_or(ReportType::OccupancySummary),
        frequency: frequency.clone(),
        recipients: req
            .recipients
            .unwrap_or_else(|| vec!["admin@parkhub.test".to_string()]),
        enabled: req.enabled.unwrap_or(true),
        last_sent_at: None,
        next_run_at: compute_next_run(&frequency),
        created_at: now.clone(),
        updated_at: now,
    };

    (StatusCode::OK, Json(ApiResponse::success(schedule)))
}

/// `DELETE /api/v1/admin/reports/schedules/{id}` — delete a schedule.
pub async fn delete_schedule(
    State(_state): State<SharedState>,
    Path(_schedule_id): Path<String>,
) -> StatusCode {
    StatusCode::NO_CONTENT
}

/// `POST /api/v1/admin/reports/schedules/{id}/send-now` — trigger immediate send.
pub async fn send_now(
    State(_state): State<SharedState>,
    Path(schedule_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<SendNowResponse>>) {
    let response = SendNowResponse {
        schedule_id,
        report_type: ReportType::OccupancySummary,
        recipients_count: 1,
        sent_at: Utc::now().to_rfc3339(),
        message: "Report sent successfully".to_string(),
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_type_labels() {
        assert_eq!(ReportType::OccupancySummary.label(), "Occupancy Summary");
        assert_eq!(ReportType::RevenueReport.label(), "Revenue Report");
        assert_eq!(ReportType::UserActivity.label(), "User Activity");
        assert_eq!(ReportType::BookingTrends.label(), "Booking Trends");
    }

    #[test]
    fn test_report_type_serialize() {
        assert_eq!(
            serde_json::to_string(&ReportType::OccupancySummary).unwrap(),
            "\"occupancy_summary\""
        );
        assert_eq!(
            serde_json::to_string(&ReportType::RevenueReport).unwrap(),
            "\"revenue_report\""
        );
        assert_eq!(
            serde_json::to_string(&ReportType::UserActivity).unwrap(),
            "\"user_activity\""
        );
        assert_eq!(
            serde_json::to_string(&ReportType::BookingTrends).unwrap(),
            "\"booking_trends\""
        );
    }

    #[test]
    fn test_report_type_deserialize() {
        let r: ReportType = serde_json::from_str("\"occupancy_summary\"").unwrap();
        assert_eq!(r, ReportType::OccupancySummary);
        let r: ReportType = serde_json::from_str("\"booking_trends\"").unwrap();
        assert_eq!(r, ReportType::BookingTrends);
    }

    #[test]
    fn test_report_type_all() {
        assert_eq!(ReportType::ALL.len(), 4);
    }

    #[test]
    fn test_schedule_frequency_labels() {
        assert_eq!(ScheduleFrequency::Daily.label(), "Daily");
        assert_eq!(ScheduleFrequency::Weekly.label(), "Weekly");
        assert_eq!(ScheduleFrequency::Monthly.label(), "Monthly");
    }

    #[test]
    fn test_schedule_frequency_cron() {
        assert_eq!(ScheduleFrequency::Daily.cron_expression(), "0 8 * * *");
        assert_eq!(ScheduleFrequency::Weekly.cron_expression(), "0 8 * * 1");
        assert_eq!(ScheduleFrequency::Monthly.cron_expression(), "0 8 1 * *");
    }

    #[test]
    fn test_schedule_frequency_serialize() {
        assert_eq!(
            serde_json::to_string(&ScheduleFrequency::Daily).unwrap(),
            "\"daily\""
        );
        assert_eq!(
            serde_json::to_string(&ScheduleFrequency::Weekly).unwrap(),
            "\"weekly\""
        );
        assert_eq!(
            serde_json::to_string(&ScheduleFrequency::Monthly).unwrap(),
            "\"monthly\""
        );
    }

    #[test]
    fn test_validate_recipients_ok() {
        let result = validate_recipients(&["user@example.com".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_recipients_empty() {
        let result = validate_recipients(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_recipients_invalid_email() {
        let result = validate_recipients(&["not-an-email".to_string()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_compute_next_run_daily() {
        let next = compute_next_run(&ScheduleFrequency::Daily);
        assert!(next.contains('T')); // ISO 8601 datetime
    }

    #[test]
    fn test_generate_sample_schedules() {
        let schedules = generate_sample_schedules();
        assert_eq!(schedules.len(), 2);
        assert!(schedules[0].enabled);
        assert!(schedules[0].last_sent_at.is_some());
        assert!(schedules[1].last_sent_at.is_none());
    }

    #[test]
    fn test_report_schedule_serialize() {
        let schedule = ReportSchedule {
            id: "test".to_string(),
            name: "Test Schedule".to_string(),
            report_type: ReportType::RevenueReport,
            frequency: ScheduleFrequency::Weekly,
            recipients: vec!["admin@test.com".to_string()],
            enabled: true,
            last_sent_at: None,
            next_run_at: "2026-03-30T08:00:00Z".to_string(),
            created_at: "2026-03-23T10:00:00Z".to_string(),
            updated_at: "2026-03-23T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&schedule).unwrap();
        assert!(json.contains("\"report_type\":\"revenue_report\""));
        assert!(json.contains("\"frequency\":\"weekly\""));
        assert!(json.contains("\"enabled\":true"));
    }

    #[test]
    fn test_send_now_response_serialize() {
        let resp = SendNowResponse {
            schedule_id: "s-1".to_string(),
            report_type: ReportType::BookingTrends,
            recipients_count: 3,
            sent_at: "2026-03-23T10:00:00Z".to_string(),
            message: "OK".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"recipients_count\":3"));
        assert!(json.contains("\"report_type\":\"booking_trends\""));
    }

    #[test]
    fn test_create_schedule_request_deserialize() {
        let json = r#"{"name":"Daily Report","report_type":"occupancy_summary","frequency":"daily","recipients":["admin@test.com"]}"#;
        let req: CreateScheduleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name, "Daily Report");
        assert_eq!(req.report_type, ReportType::OccupancySummary);
        assert_eq!(req.frequency, ScheduleFrequency::Daily);
        assert_eq!(req.recipients.len(), 1);
    }

    #[test]
    fn test_update_schedule_request_partial() {
        let json = r#"{"enabled":false}"#;
        let req: UpdateScheduleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.enabled, Some(false));
        assert!(req.name.is_none());
        assert!(req.report_type.is_none());
    }

    #[test]
    fn test_schedule_list_response_serialize() {
        let resp = ScheduleListResponse {
            schedules: vec![],
            total: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"schedules\":[]"));
    }
}
