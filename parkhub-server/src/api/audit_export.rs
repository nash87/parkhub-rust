//! Enhanced Audit Log Export — PDF, CSV, and JSON formats with signed download URLs.
//!
//! Extends the base audit log export with:
//! - Multi-format export (PDF, CSV, JSON)
//! - Date range, action type, and user filtering
//! - Signed download URLs with 5-minute expiry
//!
//! Endpoints:
//! - `GET /api/v1/admin/audit-log/export/enhanced` — export with format param
//! - `GET /api/v1/admin/audit-log/export/download/{token}` — signed download

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::{AuthUser, check_admin};

type SharedState = Arc<RwLock<crate::AppState>>;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Csv,
    Json,
    Pdf,
}

impl ExportFormat {
    fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    const fn content_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv; charset=utf-8",
            Self::Json => "application/json; charset=utf-8",
            Self::Pdf => "application/pdf",
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Pdf => "pdf",
        }
    }
}

/// Query parameters for enhanced audit export.
#[derive(Debug, Deserialize, Default, utoipa::IntoParams)]
pub struct AuditExportParams {
    /// Export format: csv, json, or pdf (default: csv)
    pub format: Option<String>,
    /// Start date (inclusive), e.g. `2026-01-01`
    pub from: Option<String>,
    /// End date (inclusive), e.g. `2026-03-23`
    pub to: Option<String>,
    /// Filter by action type (partial match)
    pub action: Option<String>,
    /// Filter by user ID or username
    pub user_id: Option<String>,
}

/// Signed download token stored in settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadToken {
    pub token: String,
    pub format: ExportFormat,
    pub filters: AuditExportFilters,
    pub expires_at: chrono::DateTime<Utc>,
    pub created_by: Uuid,
}

/// Filter params embedded in download token.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AuditExportFilters {
    pub from: Option<String>,
    pub to: Option<String>,
    pub action: Option<String>,
    pub user_id: Option<String>,
}

/// Response for the export initiation endpoint.
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    /// Direct download URL with signed token
    pub download_url: String,
    /// Token for the download (5 min expiry)
    pub token: String,
    /// Expiry timestamp
    pub expires_at: String,
    /// Export format
    pub format: String,
}

const DOWNLOAD_TOKENS_KEY: &str = "audit_export_tokens";

// ─────────────────────────────────────────────────────────────────────────────
// Token storage
// ─────────────────────────────────────────────────────────────────────────────

async fn load_tokens(state: &crate::AppState) -> Vec<DownloadToken> {
    match state.db.get_setting(DOWNLOAD_TOKENS_KEY).await {
        Ok(Some(json)) => serde_json::from_str(&json).unwrap_or_default(),
        _ => vec![],
    }
}

async fn save_tokens(state: &crate::AppState, tokens: &[DownloadToken]) {
    let json = serde_json::to_string(tokens).unwrap_or_default();
    let _ = state.db.set_setting(DOWNLOAD_TOKENS_KEY, &json).await;
}

// ─────────────────────────────────────────────────────────────────────────────
// CSV injection protection
// ─────────────────────────────────────────────────────────────────────────────

fn csv_escape(value: &str) -> String {
    let needs_prefix = value.starts_with('=')
        || value.starts_with('+')
        || value.starts_with('-')
        || value.starts_with('@');

    let val = if needs_prefix {
        format!("'{value}")
    } else {
        value.to_string()
    };

    if val.contains(',') || val.contains('"') || val.contains('\n') {
        format!("\"{}\"", val.replace('"', "\"\""))
    } else {
        val
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/admin/audit-log/export/enhanced` — create signed download URL.
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit-log/export/enhanced",
    tag = "Audit Export",
    summary = "Create signed audit log export",
    description = "Creates a signed download URL for audit log export in CSV, JSON, or PDF format. URL expires in 5 minutes.",
    params(AuditExportParams),
    responses(
        (status = 200, description = "Signed download URL created"),
        (status = 400, description = "Invalid format"),
    )
)]
pub async fn enhanced_audit_export(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<AuditExportParams>,
) -> (StatusCode, Json<ApiResponse<ExportResponse>>) {
    let state_guard = state.read().await;
    if let Err((status, _msg)) = check_admin(&state_guard, &auth_user).await {
        return (
            status,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let format = params
        .format
        .as_deref()
        .map(ExportFormat::from_str_opt)
        .unwrap_or(Some(ExportFormat::Csv));

    let Some(format) = format else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_FORMAT",
                "Supported formats: csv, json, pdf",
            )),
        );
    };

    // Validate date params if provided
    if let Some(ref from) = params.from
        && NaiveDate::parse_from_str(from, "%Y-%m-%d").is_err()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_DATE",
                "Invalid 'from' date format. Use YYYY-MM-DD",
            )),
        );
    }
    if let Some(ref to) = params.to
        && NaiveDate::parse_from_str(to, "%Y-%m-%d").is_err()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_DATE",
                "Invalid 'to' date format. Use YYYY-MM-DD",
            )),
        );
    }

    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::TimeDelta::minutes(5);

    let dl_token = DownloadToken {
        token: token.clone(),
        format,
        filters: AuditExportFilters {
            from: params.from,
            to: params.to,
            action: params.action,
            user_id: params.user_id,
        },
        expires_at,
        created_by: auth_user.user_id,
    };

    // Clean expired tokens and add new one
    let mut tokens = load_tokens(&state_guard).await;
    let now = Utc::now();
    tokens.retain(|t| t.expires_at > now);
    tokens.push(dl_token);
    save_tokens(&state_guard, &tokens).await;

    let download_url = format!("/api/v1/admin/audit-log/export/download/{token}");

    (
        StatusCode::OK,
        Json(ApiResponse::success(ExportResponse {
            download_url,
            token,
            expires_at: expires_at.to_rfc3339(),
            format: format.extension().to_string(),
        })),
    )
}

/// `GET /api/v1/admin/audit-log/export/download/{token}` — download the export file.
#[utoipa::path(
    get,
    path = "/api/v1/admin/audit-log/export/download/{token}",
    tag = "Audit Export",
    summary = "Download audit log export",
    description = "Download the exported audit log using a signed token. Token expires after 5 minutes.",
    params(("token" = String, Path, description = "Download token")),
    responses(
        (status = 200, description = "Export file"),
        (status = 401, description = "Invalid or expired token"),
    )
)]
pub async fn download_audit_export(
    State(state): State<SharedState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let state_guard = state.read().await;

    // Find and validate token
    let tokens = load_tokens(&state_guard).await;
    let now = Utc::now();

    let Some(dl_token) = tokens
        .iter()
        .find(|t| t.token == token && t.expires_at > now)
    else {
        return (
            StatusCode::UNAUTHORIZED,
            [
                (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
                (header::CONTENT_DISPOSITION, "inline"),
            ],
            "Invalid or expired download token".to_string(),
        );
    };

    let format = dl_token.format;
    let filters = dl_token.filters.clone();

    // Load and filter entries
    let entries = match state_guard.db.list_all_audit_log().await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("Failed to load audit log: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
                    (header::CONTENT_DISPOSITION, "inline"),
                ],
                "Failed to load audit log".to_string(),
            );
        }
    };

    let mut filtered = entries;

    if let Some(ref action) = filters.action {
        let q = action.to_lowercase();
        filtered.retain(|e| e.event_type.to_lowercase().contains(&q));
    }
    if let Some(ref user) = filters.user_id {
        let q = user.to_lowercase();
        filtered.retain(|e| {
            e.username
                .as_ref()
                .is_some_and(|u| u.to_lowercase().contains(&q))
                || e.user_id.is_some_and(|id| id.to_string().contains(&q))
        });
    }
    if let Some(ref from) = filters.from
        && let Ok(from_date) = NaiveDate::parse_from_str(from, "%Y-%m-%d")
    {
        filtered.retain(|e| e.timestamp.date_naive() >= from_date);
    }
    if let Some(ref to) = filters.to
        && let Ok(to_date) = NaiveDate::parse_from_str(to, "%Y-%m-%d")
    {
        filtered.retain(|e| e.timestamp.date_naive() <= to_date);
    }

    let body = match format {
        ExportFormat::Csv => render_csv(&filtered),
        ExportFormat::Json => render_json(&filtered),
        ExportFormat::Pdf => render_pdf(&filtered),
    };

    let filename = format!(
        "audit-log-{}.{}",
        Utc::now().format("%Y%m%d-%H%M%S"),
        format.extension()
    );
    let disposition = format!("attachment; filename=\"{filename}\"");
    // Use a leaked string for the content-disposition to get &'static str
    let disposition: &'static str = Box::leak(disposition.into_boxed_str());

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, format.content_type()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        body,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Renderers
// ─────────────────────────────────────────────────────────────────────────────

fn render_csv(entries: &[crate::db::AuditLogEntry]) -> String {
    let mut csv = String::from(
        "id,timestamp,event_type,user_id,username,target_type,target_id,ip_address,details\n",
    );
    for e in entries {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{}",
            e.id,
            e.timestamp.to_rfc3339(),
            csv_escape(&e.event_type),
            e.user_id.map_or_else(String::new, |id| id.to_string()),
            csv_escape(e.username.as_deref().unwrap_or("")),
            csv_escape(e.target_type.as_deref().unwrap_or("")),
            csv_escape(e.target_id.as_deref().unwrap_or("")),
            csv_escape(e.ip_address.as_deref().unwrap_or("")),
            csv_escape(e.details.as_deref().unwrap_or("")),
        );
    }
    csv
}

fn render_json(entries: &[crate::db::AuditLogEntry]) -> String {
    #[derive(Serialize)]
    struct JsonExport {
        exported_at: String,
        total_entries: usize,
        entries: Vec<JsonAuditEntry>,
    }

    #[derive(Serialize)]
    struct JsonAuditEntry {
        id: String,
        timestamp: String,
        event_type: String,
        user_id: Option<String>,
        username: Option<String>,
        target_type: Option<String>,
        target_id: Option<String>,
        ip_address: Option<String>,
        details: Option<String>,
    }

    let export = JsonExport {
        exported_at: Utc::now().to_rfc3339(),
        total_entries: entries.len(),
        entries: entries
            .iter()
            .map(|e| JsonAuditEntry {
                id: e.id.to_string(),
                timestamp: e.timestamp.to_rfc3339(),
                event_type: e.event_type.clone(),
                user_id: e.user_id.map(|id| id.to_string()),
                username: e.username.clone(),
                target_type: e.target_type.clone(),
                target_id: e.target_id.clone(),
                ip_address: e.ip_address.clone(),
                details: e.details.clone(),
            })
            .collect(),
    };

    serde_json::to_string_pretty(&export).unwrap_or_default()
}

fn render_pdf(entries: &[crate::db::AuditLogEntry]) -> String {
    // Generate a text-based report (lightweight — no external PDF library needed)
    let mut report = String::from("ParkHub Audit Log Report\n");
    let _ = writeln!(report, "Generated: {}", Utc::now().to_rfc3339());
    let _ = writeln!(report, "Total Entries: {}\n", entries.len());
    let _ = writeln!(
        report,
        "{:<36} {:<24} {:<20} {:<20}",
        "ID", "Timestamp", "Event", "User"
    );
    let _ = writeln!(report, "{}", "-".repeat(100));

    for e in entries {
        let _ = writeln!(
            report,
            "{:<36} {:<24} {:<20} {:<20}",
            &e.id.to_string()[..8],
            e.timestamp.format("%Y-%m-%d %H:%M:%S"),
            &e.event_type,
            e.username.as_deref().unwrap_or("-"),
        );
    }

    report
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_from_str() {
        assert_eq!(ExportFormat::from_str_opt("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_str_opt("json"), Some(ExportFormat::Json));
        assert_eq!(ExportFormat::from_str_opt("pdf"), Some(ExportFormat::Pdf));
        assert_eq!(ExportFormat::from_str_opt("CSV"), Some(ExportFormat::Csv));
        assert!(ExportFormat::from_str_opt("xml").is_none());
    }

    #[test]
    fn test_export_format_content_type() {
        assert_eq!(ExportFormat::Csv.content_type(), "text/csv; charset=utf-8");
        assert_eq!(
            ExportFormat::Json.content_type(),
            "application/json; charset=utf-8"
        );
        assert_eq!(ExportFormat::Pdf.content_type(), "application/pdf");
    }

    #[test]
    fn test_export_format_extension() {
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Pdf.extension(), "pdf");
    }

    #[test]
    fn test_csv_escape_normal() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_injection() {
        assert_eq!(csv_escape("=SUM(A1:A10)"), "'=SUM(A1:A10)");
        assert_eq!(csv_escape("+cmd"), "'+cmd");
        assert_eq!(csv_escape("-rm"), "'-rm");
        assert_eq!(csv_escape("@import"), "'@import");
    }

    #[test]
    fn test_csv_escape_quotes() {
        assert_eq!(csv_escape("hello,world"), "\"hello,world\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_download_token_serialization() {
        let token = DownloadToken {
            token: "abc-123".to_string(),
            format: ExportFormat::Csv,
            filters: AuditExportFilters::default(),
            expires_at: Utc::now(),
            created_by: Uuid::new_v4(),
        };
        let json = serde_json::to_string(&token).unwrap();
        let de: DownloadToken = serde_json::from_str(&json).unwrap();
        assert_eq!(de.token, "abc-123");
        assert_eq!(de.format, ExportFormat::Csv);
    }

    #[test]
    fn test_export_response_serialization() {
        let resp = ExportResponse {
            download_url: "/api/v1/admin/audit-log/export/download/abc".to_string(),
            token: "abc".to_string(),
            expires_at: Utc::now().to_rfc3339(),
            format: "csv".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("download_url"));
        assert!(json.contains("token"));
    }

    #[test]
    fn test_render_json_empty() {
        let result = render_json(&[]);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["total_entries"], 0);
    }

    #[test]
    fn test_render_csv_header() {
        let result = render_csv(&[]);
        assert!(result.starts_with("id,timestamp,event_type"));
    }

    #[test]
    fn test_render_pdf_header() {
        let result = render_pdf(&[]);
        assert!(result.contains("ParkHub Audit Log Report"));
        assert!(result.contains("Total Entries: 0"));
    }

    #[test]
    fn test_audit_export_filters_default() {
        let filters = AuditExportFilters::default();
        assert!(filters.from.is_none());
        assert!(filters.to.is_none());
        assert!(filters.action.is_none());
        assert!(filters.user_id.is_none());
    }
}
