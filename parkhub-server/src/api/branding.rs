//! Branding configuration and logo endpoints.
//!
//! Stores branding config in admin settings under keys:
//!   - `branding_app_name`
//!   - `branding_primary_color`
//!   - `branding_logo_base64`

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    Extension,
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
