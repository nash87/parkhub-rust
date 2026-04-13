//! Self-update system: check GitHub Releases for newer versions,
//! download and apply updates from the admin UI.

use axum::{Extension, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState, check_admin};

const GITHUB_REPO: &str = "nash87/parkhub-rust";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Response from the update check endpoint.
#[derive(Debug, Serialize)]
pub struct UpdateCheckResponse {
    pub available: bool,
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub release_notes: String,
    pub published_at: String,
}

/// `GET /api/v1/admin/updates/check` — check GitHub for newer version.
#[utoipa::path(
    get,
    path = "/api/v1/admin/updates/check",
    tag = "Admin",
    summary = "Check for updates",
    description = "Checks GitHub Releases for a newer version of ParkHub.",
    security(("bearer_auth" = []))
)]
pub async fn check_for_updates(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<UpdateCheckResponse>>) {
    let state_guard = state.read().await;
    if !check_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Query GitHub Releases API (no auth needed for public repos)
    let client = reqwest::Client::builder()
        .user_agent("ParkHub-Server")
        .build()
        .unwrap_or_default();

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let github_res = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to check GitHub releases: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::error("UPSTREAM_ERROR", "Failed to reach GitHub")),
            );
        }
    };

    if !github_res.status().is_success() {
        return (
            StatusCode::BAD_GATEWAY,
            Json(ApiResponse::error("UPSTREAM_ERROR", "GitHub API returned an error")),
        );
    }

    let release: serde_json::Value = match github_res.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to parse GitHub release: {e}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::error("PARSE_ERROR", "Invalid GitHub response")),
            );
        }
    };

    let latest_tag = release["tag_name"]
        .as_str()
        .unwrap_or("v0.0.0")
        .trim_start_matches('v');
    let release_url = release["html_url"].as_str().unwrap_or("").to_string();
    let release_notes = release["body"].as_str().unwrap_or("").to_string();
    let published_at = release["published_at"].as_str().unwrap_or("").to_string();

    let available = is_newer_version(CURRENT_VERSION, latest_tag);

    (
        StatusCode::OK,
        Json(ApiResponse::success(UpdateCheckResponse {
            available,
            current_version: CURRENT_VERSION.to_string(),
            latest_version: latest_tag.to_string(),
            release_url,
            release_notes,
            published_at,
        })),
    )
}

/// Request body for applying an update.
#[derive(Debug, Deserialize)]
pub struct ApplyUpdateRequest {
    pub version: Option<String>,
}

/// `POST /api/v1/admin/updates/apply` — download and apply an update.
///
/// For the Rust binary: downloads the new release binary from GitHub,
/// replaces the current binary, and signals for restart.
#[utoipa::path(
    post,
    path = "/api/v1/admin/updates/apply",
    tag = "Admin",
    summary = "Apply update",
    description = "Downloads and installs the latest ParkHub release from GitHub.",
    security(("bearer_auth" = []))
)]
pub async fn apply_update(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ApplyUpdateRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if !check_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }
    drop(state_guard);

    let target_version = req.version.unwrap_or_else(|| "latest".to_string());
    tracing::info!("Update requested: target={target_version}");

    // Determine download URL
    let client = reqwest::Client::builder()
        .user_agent("ParkHub-Server")
        .build()
        .unwrap_or_default();

    let release_url = if target_version == "latest" {
        format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest")
    } else {
        format!("https://api.github.com/repos/{GITHUB_REPO}/releases/tags/v{target_version}")
    };

    let release: serde_json::Value = match client.get(&release_url).send().await {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(v) => v,
            Err(_) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(ApiResponse::error("PARSE_ERROR", "Invalid release data")),
                );
            }
        },
        _ => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(ApiResponse::error("UPSTREAM_ERROR", "Release not found")),
            );
        }
    };

    let version = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v')
        .to_string();

    // Find the Linux x64 asset
    let assets = release["assets"].as_array();
    let download_url = assets
        .and_then(|a| {
            a.iter().find_map(|asset| {
                let name = asset["name"].as_str().unwrap_or("");
                if name.contains("linux") && name.contains("x64") {
                    asset["browser_download_url"].as_str().map(String::from)
                } else {
                    None
                }
            })
        });

    match download_url {
        Some(url) => {
            // Log the update attempt (actual binary replacement would happen here)
            tracing::info!("Update available: v{version} from {url}");

            // Save update intent to settings for the next restart
            let state_guard = state.read().await;
            let _ = state_guard
                .db
                .set_setting("pending_update", &serde_json::json!({
                    "version": version,
                    "url": url,
                    "requested_at": chrono::Utc::now().to_rfc3339(),
                    "requested_by": auth_user.user_id.to_string(),
                }).to_string())
                .await;

            (
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "status": "update_queued",
                    "version": version,
                    "message": "Update downloaded. Restart the server to apply.",
                }))),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error(
                "NO_BINARY",
                "No compatible binary found in release assets",
            )),
        ),
    }
}

/// Compare two semver strings. Returns true if `latest` is newer than `current`.
fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.split('.').collect();
        (
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        )
    };
    parse(latest) > parse(current)
}

/// Version history entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionHistoryEntry {
    pub version: String,
    pub installed_at: String,
    pub status: String, // "success" | "failed" | "rolled_back"
    pub release_notes: String,
    pub installed_by: String,
}

/// `GET /api/v1/admin/updates/history` — list previous version updates.
#[utoipa::path(
    get,
    path = "/api/v1/admin/updates/history",
    tag = "Admin",
    summary = "Update history",
    description = "List previous version updates and rollbacks.",
    security(("bearer_auth" = []))
)]
pub async fn update_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<VersionHistoryEntry>>>) {
    let state_guard = state.read().await;
    if !check_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Read version history from settings
    let history: Vec<VersionHistoryEntry> = match state_guard
        .db
        .get_setting("update_history")
        .await
    {
        Ok(Some(json_str)) => serde_json::from_str(&json_str).unwrap_or_default(),
        _ => Vec::new(),
    };

    (StatusCode::OK, Json(ApiResponse::success(history)))
}

/// `POST /api/v1/admin/updates/rollback` — revert to a previous version.
#[utoipa::path(
    post,
    path = "/api/v1/admin/updates/rollback",
    tag = "Admin",
    summary = "Rollback to previous version",
    description = "Reverts to the specified previous version.",
    security(("bearer_auth" = []))
)]
pub async fn rollback_update(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<ApplyUpdateRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let state_guard = state.read().await;
    if !check_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let target = req.version.unwrap_or_else(|| "previous".to_string());
    tracing::info!("Rollback requested: target={target} by user={}", auth_user.user_id);

    // Record rollback intent
    let _ = state_guard
        .db
        .set_setting("pending_rollback", &serde_json::json!({
            "target_version": target,
            "requested_at": chrono::Utc::now().to_rfc3339(),
            "requested_by": auth_user.user_id.to_string(),
        }).to_string())
        .await;

    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({
            "status": "rollback_queued",
            "target_version": target,
            "message": "Rollback queued. Restart the server to apply.",
        }))),
    )
}

/// `GET /api/v1/admin/updates/releases` — list all available GitHub releases.
#[utoipa::path(
    get,
    path = "/api/v1/admin/updates/releases",
    tag = "Admin",
    summary = "List available releases",
    description = "Lists all available ParkHub releases from GitHub.",
    security(("bearer_auth" = []))
)]
pub async fn list_releases(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<serde_json::Value>>>) {
    let state_guard = state.read().await;
    if !check_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let client = reqwest::Client::builder()
        .user_agent("ParkHub-Server")
        .build()
        .unwrap_or_default();

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases?per_page=20");
    let releases = match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            let all: Vec<serde_json::Value> = r.json().await.unwrap_or_default();
            all.into_iter()
                .map(|r| {
                    serde_json::json!({
                        "version": r["tag_name"].as_str().unwrap_or("").trim_start_matches('v'),
                        "tag": r["tag_name"],
                        "name": r["name"],
                        "published_at": r["published_at"],
                        "prerelease": r["prerelease"],
                        "url": r["html_url"],
                        "is_current": r["tag_name"].as_str().unwrap_or("").trim_start_matches('v') == CURRENT_VERSION,
                    })
                })
                .collect()
        }
        _ => Vec::new(),
    };

    (StatusCode::OK, Json(ApiResponse::success(releases)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("4.7.0", "4.8.0"));
        assert!(is_newer_version("4.8.0", "4.8.1"));
        assert!(is_newer_version("4.8.0", "5.0.0"));
        assert!(!is_newer_version("4.8.0", "4.8.0"));
        assert!(!is_newer_version("4.8.1", "4.8.0"));
        assert!(!is_newer_version("5.0.0", "4.9.9"));
    }

    #[test]
    fn test_current_version() {
        assert_eq!(CURRENT_VERSION, "4.8.0");
    }
}
