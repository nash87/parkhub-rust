//! Branding configuration and logo endpoints.
//!
//! Stores branding config in admin settings under keys:
//!   - `branding_app_name`
//!   - `branding_primary_color`
//!   - `branding_logo_base64`

use axum::{
    Extension,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Json, Response},
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

const MAX_LOGO_BYTES: usize = 2 * 1024 * 1024; // 2 MB raw

#[derive(Debug, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub app_name: String,
    pub primary_color: String,
    pub logo_url: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BrandingUpdate {
    pub app_name: Option<String>,
    pub primary_color: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LogoUpload {
    /// Base64-encoded image data, optionally with a `data:<mime>;base64,` prefix.
    pub logo: String,
}

fn strip_data_uri(input: &str) -> &str {
    input
        .find(";base64,")
        .map_or(input, |pos| &input[pos + 8..])
}

fn detect_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        Some("image/jpeg")
    } else if bytes.len() >= 4
        && bytes[0] == 0x89
        && bytes[1] == 0x50
        && bytes[2] == 0x4E
        && bytes[3] == 0x47
    {
        Some("image/png")
    } else {
        None
    }
}

/// `GET /api/v1/admin/branding` — read current branding config.
#[utoipa::path(
    get,
    path = "/api/v1/admin/branding",
    tag = "Admin",
    summary = "Get branding configuration",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Branding config"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_get_branding(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<BrandingConfig>>) {
    let state_guard = state.read().await;

    // Admin-only
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    let app_name = state_guard
        .db
        .get_setting("branding_app_name")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "ParkHub".to_string());

    let primary_color = state_guard
        .db
        .get_setting("branding_primary_color")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "#2563eb".to_string());

    let logo_url = state_guard
        .db
        .get_setting("branding_logo_base64")
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            if v.is_empty() {
                None
            } else {
                Some("/api/v1/branding/logo".to_string())
            }
        });

    (
        StatusCode::OK,
        Json(ApiResponse::success(BrandingConfig {
            app_name,
            primary_color,
            logo_url,
        })),
    )
}

/// `PUT /api/v1/admin/branding` — update branding config.
#[utoipa::path(
    put,
    path = "/api/v1/admin/branding",
    tag = "Admin",
    summary = "Update branding configuration",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Updated branding config"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_update_branding(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<BrandingUpdate>,
) -> (StatusCode, Json<ApiResponse<BrandingConfig>>) {
    let state_guard = state.read().await;

    // Admin-only
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    if let Some(ref name) = req.app_name {
        if let Err(e) = state_guard.db.set_setting("branding_app_name", name).await {
            tracing::error!("Failed to save branding_app_name: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to save branding",
                )),
            );
        }
    }

    if let Some(ref color) = req.primary_color {
        if let Err(e) = state_guard
            .db
            .set_setting("branding_primary_color", color)
            .await
        {
            tracing::error!("Failed to save branding_primary_color: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to save branding",
                )),
            );
        }
    }

    let app_name = state_guard
        .db
        .get_setting("branding_app_name")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "ParkHub".to_string());

    let primary_color = state_guard
        .db
        .get_setting("branding_primary_color")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "#2563eb".to_string());

    let logo_url = state_guard
        .db
        .get_setting("branding_logo_base64")
        .await
        .ok()
        .flatten()
        .and_then(|v| {
            if v.is_empty() {
                None
            } else {
                Some("/api/v1/branding/logo".to_string())
            }
        });

    (
        StatusCode::OK,
        Json(ApiResponse::success(BrandingConfig {
            app_name,
            primary_color,
            logo_url,
        })),
    )
}

/// `POST /api/v1/admin/branding/logo` — upload a logo (base64, max 2MB, PNG/JPEG).
#[utoipa::path(
    post,
    path = "/api/v1/admin/branding/logo",
    tag = "Admin",
    summary = "Upload branding logo",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logo uploaded"),
        (status = 400, description = "Invalid image or too large"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn admin_upload_logo(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<LogoUpload>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;

    // Admin-only
    match state_guard
        .db
        .get_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(Some(u)) if u.role == UserRole::Admin || u.role == UserRole::SuperAdmin => {}
        _ => {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
            );
        }
    }

    let b64 = strip_data_uri(&req.logo);

    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID_INPUT", "Invalid base64 data")),
        );
    };

    if raw_bytes.len() > MAX_LOGO_BYTES {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "PAYLOAD_TOO_LARGE",
                "Logo exceeds 2 MB limit",
            )),
        );
    }

    if detect_mime(&raw_bytes).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "INVALID_INPUT",
                "Unsupported image format. Only JPEG and PNG are accepted.",
            )),
        );
    }

    if let Err(e) = state_guard
        .db
        .set_setting("branding_logo_base64", b64)
        .await
    {
        tracing::error!("Failed to save branding logo: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save logo")),
        );
    }

    tracing::info!(bytes = raw_bytes.len(), "Branding logo uploaded");
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "url": "/api/v1/branding/logo",
            "bytes": raw_bytes.len()
        }))),
    )
}

/// `GET /api/v1/branding/logo` — serve the current branding logo (public, cached).
#[utoipa::path(
    get,
    path = "/api/v1/branding/logo",
    tag = "Branding",
    summary = "Get branding logo",
    description = "Returns the stored logo image with appropriate Content-Type and Cache-Control headers.",
    responses(
        (status = 200, description = "Logo image"),
        (status = 404, description = "No logo configured")
    )
)]
pub async fn get_branding_logo(State(state): State<SharedState>) -> Response {
    let state_guard = state.read().await;

    let stored = match state_guard.db.get_setting("branding_logo_base64").await {
        Ok(Some(v)) if !v.is_empty() => v,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("NOT_FOUND", "No logo configured")),
            )
                .into_response();
        }
    };

    drop(state_guard);

    let b64 = strip_data_uri(&stored);

    let Ok(raw_bytes) = base64::engine::general_purpose::STANDARD.decode(b64) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                "SERVER_ERROR",
                "Corrupt logo data",
            )),
        )
            .into_response();
    };

    let content_type = detect_mime(&raw_bytes).unwrap_or("application/octet-stream");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(raw_bytes))
        .unwrap_or_else(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to build response",
            )
                .into_response()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HEAD: strip_data_uri ────────────────────────────────────────────────

    #[test]
    fn strip_data_uri_with_prefix() {
        let input = "data:image/png;base64,iVBOR...";
        assert_eq!(strip_data_uri(input), "iVBOR...");
    }

    #[test]
    fn strip_data_uri_with_jpeg_prefix() {
        let input = "data:image/jpeg;base64,/9j/4AAQ";
        assert_eq!(strip_data_uri(input), "/9j/4AAQ");
    }

    #[test]
    fn strip_data_uri_no_prefix() {
        let input = "iVBORw0KGgoAAAANS";
        assert_eq!(strip_data_uri(input), "iVBORw0KGgoAAAANS");
    }

    #[test]
    fn strip_data_uri_empty_string() {
        assert_eq!(strip_data_uri(""), "");
    }

    #[test]
    fn strip_data_uri_only_prefix() {
        let input = "data:image/png;base64,";
        assert_eq!(strip_data_uri(input), "");
    }

    #[test]
    fn strip_data_uri_semicolon_no_base64() {
        let input = "data:image/png;charset=utf8,ABC";
        // No ";base64," -> returns input unchanged
        assert_eq!(strip_data_uri(input), input);
    }

    // ── HEAD: detect_mime ───────────────────────────────────────────────────

    #[test]
    fn detect_mime_jpeg() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_mime(&jpeg_header), Some("image/jpeg"));
    }

    #[test]
    fn detect_mime_jpeg_minimal() {
        let jpeg_min = [0xFF, 0xD8, 0xFF];
        assert_eq!(detect_mime(&jpeg_min), Some("image/jpeg"));
    }

    #[test]
    fn detect_mime_png() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert_eq!(detect_mime(&png_header), Some("image/png"));
    }

    #[test]
    fn detect_mime_png_minimal() {
        let png_min = [0x89, 0x50, 0x4E, 0x47];
        assert_eq!(detect_mime(&png_min), Some("image/png"));
    }

    #[test]
    fn detect_mime_unknown_format() {
        let gif_header = [0x47, 0x49, 0x46, 0x38];
        assert_eq!(detect_mime(&gif_header), None);
    }

    #[test]
    fn detect_mime_empty_bytes() {
        assert_eq!(detect_mime(&[]), None);
    }

    #[test]
    fn detect_mime_too_short_for_jpeg() {
        assert_eq!(detect_mime(&[0xFF, 0xD8]), None);
    }

    #[test]
    fn detect_mime_too_short_for_png() {
        assert_eq!(detect_mime(&[0x89, 0x50, 0x4E]), None);
    }

    #[test]
    fn detect_mime_single_byte() {
        assert_eq!(detect_mime(&[0xFF]), None);
    }

    // ── HEAD: BrandingConfig serde ──────────────────────────────────────────

    #[test]
    fn branding_config_serde_with_logo() {
        let config = BrandingConfig {
            app_name: "MyParking".into(),
            primary_color: "#ff0000".into(),
            logo_url: Some("/api/v1/branding/logo".into()),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.app_name, "MyParking");
        assert_eq!(parsed.logo_url.unwrap(), "/api/v1/branding/logo");
    }

    #[test]
    fn branding_config_serde_without_logo() {
        let config = BrandingConfig {
            app_name: "ParkHub".into(),
            primary_color: "#2563eb".into(),
            logo_url: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.logo_url.is_none());
    }

    #[test]
    fn branding_update_deserialization() {
        let json = r##"{"app_name":"New Name","primary_color":"#123456"}"##;
        let update: BrandingUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.app_name.unwrap(), "New Name");
        assert_eq!(update.primary_color.unwrap(), "#123456");
    }

    #[test]
    fn branding_update_partial() {
        let json = r#"{"app_name":"Only Name"}"#;
        let update: BrandingUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.app_name.unwrap(), "Only Name");
        assert!(update.primary_color.is_none());
    }

    #[test]
    fn logo_upload_deserialization() {
        let json = r#"{"logo":"data:image/png;base64,iVBOR..."}"#;
        let upload: LogoUpload = serde_json::from_str(json).unwrap();
        assert!(upload.logo.starts_with("data:"));
    }

    #[test]
    fn max_logo_bytes_is_two_mb() {
        assert_eq!(MAX_LOGO_BYTES, 2 * 1024 * 1024);
    }

    // ── Copilot: BrandingConfig serialization ───────────────────────────────

    #[test]
    fn test_branding_config_roundtrip_with_logo() {
        let cfg = BrandingConfig {
            app_name: "MyPark".to_string(),
            primary_color: "#ff0000".to_string(),
            logo_url: Some("/api/v1/branding/logo".to_string()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.app_name, "MyPark");
        assert_eq!(back.primary_color, "#ff0000");
        assert_eq!(back.logo_url, Some("/api/v1/branding/logo".to_string()));
    }

    #[test]
    fn test_branding_config_roundtrip_no_logo() {
        let cfg = BrandingConfig {
            app_name: "ParkHub".to_string(),
            primary_color: "#2563eb".to_string(),
            logo_url: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.app_name, "ParkHub");
        assert!(back.logo_url.is_none());
    }

    // ── Copilot: BrandingUpdate deserialization ─────────────────────────────

    #[test]
    fn test_branding_update_all_fields() {
        let json = r##"{"app_name":"NewName","primary_color":"#123456"}"##;
        let update: BrandingUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.app_name, Some("NewName".to_string()));
        assert_eq!(update.primary_color, Some("#123456".to_string()));
    }

    #[test]
    fn test_branding_update_partial() {
        let json = r#"{"app_name":"OnlyName"}"#;
        let update: BrandingUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.app_name, Some("OnlyName".to_string()));
        assert!(update.primary_color.is_none());
    }

    #[test]
    fn test_branding_update_empty() {
        let json = r#"{}"#;
        let update: BrandingUpdate = serde_json::from_str(json).unwrap();
        assert!(update.app_name.is_none());
        assert!(update.primary_color.is_none());
    }

    // ── Copilot: strip_data_uri ─────────────────────────────────────────────

    #[test]
    fn test_strip_data_uri_with_prefix() {
        let input = "data:image/png;base64,iVBORw0KGgo=";
        let stripped = strip_data_uri(input);
        assert_eq!(stripped, "iVBORw0KGgo=");
    }

    #[test]
    fn test_strip_data_uri_without_prefix() {
        let raw = "iVBORw0KGgoAAAANSUhEUgAAAAUA";
        assert_eq!(strip_data_uri(raw), raw);
    }

    #[test]
    fn test_strip_data_uri_jpeg_prefix() {
        let input = "data:image/jpeg;base64,/9j/4AAQSkZJRgAB";
        let stripped = strip_data_uri(input);
        assert_eq!(stripped, "/9j/4AAQSkZJRgAB");
    }

    #[test]
    fn test_strip_data_uri_empty_input() {
        assert_eq!(strip_data_uri(""), "");
    }

    // ── Copilot: detect_mime ────────────────────────────────────────────────

    #[test]
    fn test_detect_mime_jpeg_magic() {
        let bytes = [0xFF_u8, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert_eq!(detect_mime(&bytes), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_mime_png_magic() {
        // PNG magic: 0x89 P N G
        let bytes = [0x89_u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert_eq!(detect_mime(&bytes), Some("image/png"));
    }

    #[test]
    fn test_detect_mime_unknown_returns_none() {
        let bytes = [0x00_u8, 0x01, 0x02, 0x03];
        assert_eq!(detect_mime(&bytes), None);
    }

    #[test]
    fn test_detect_mime_too_short_for_jpeg() {
        // Only 2 bytes -- not enough to detect JPEG
        let bytes = [0xFF_u8, 0xD8];
        // 2 < 3, so should not match JPEG
        assert_eq!(detect_mime(&bytes), None);
    }

    #[test]
    fn test_detect_mime_too_short_for_png() {
        // Only 3 bytes -- not enough to detect PNG (needs 4)
        let bytes = [0x89_u8, 0x50, 0x4E];
        assert_eq!(detect_mime(&bytes), None);
    }

    #[test]
    fn test_detect_mime_empty_slice() {
        assert_eq!(detect_mime(&[]), None);
    }

    // ── Copilot: MAX_LOGO_BYTES constant ────────────────────────────────────

    #[test]
    fn test_max_logo_bytes_is_two_mib() {
        assert_eq!(MAX_LOGO_BYTES, 2 * 1024 * 1024);
    }

    // ── Copilot: LogoUpload deserialization ──────────────────────────────────

    #[test]
    fn test_logo_upload_roundtrip() {
        let json = r#"{"logo":"data:image/png;base64,iVBOR="}"#;
        let upload: LogoUpload = serde_json::from_str(json).unwrap();
        assert!(upload.logo.starts_with("data:image/png"));
    }
}
