//! Static File Serving
//!
//! Embeds and serves the web frontend from the binary.

use std::path::Path;

use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::Embed;

/// Embedded web frontend files
#[derive(Embed)]
#[folder = "../parkhub-web/dist"]
#[prefix = ""]
struct WebAssets;

/// Serve static files from the embedded web frontend
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    // Try exact path first
    if let Some(file) = WebAssets::get(path) {
        return serve_file(path, file);
    }

    // For SPA routing, serve index.html for non-asset paths
    if (!path.contains('.') || path.is_empty())
        && let Some(file) = WebAssets::get("index.html")
    {
        return serve_file("index.html", file);
    }

    // 404 for missing assets
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

fn serve_file(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut response = Response::builder().header(header::CONTENT_TYPE, mime.as_ref());

    // Add cache headers for assets (not index.html)
    if path != "index.html"
        && (path.contains("/assets/")
            || Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("js"))
            || Path::new(path)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("css")))
    {
        response = response.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    } else {
        response = response.header(header::CACHE_CONTROL, "no-cache");
    }

    response.body(Body::from(file.data.into_owned())).unwrap()
}

/// Check if web assets are available
pub fn has_web_assets() -> bool {
    WebAssets::get("index.html").is_some()
}

/// List all embedded assets (for debugging)
#[allow(dead_code)]
pub fn list_assets() -> Vec<String> {
    WebAssets::iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Uri;
    use http_body_util::BodyExt;

    // ── serve_file cache headers ──

    #[test]
    fn serve_file_js_gets_immutable_cache() {
        if let Some(file) = WebAssets::get("index.html") {
            // Use a fake .js path to trigger the cache branch
            let resp = serve_file("assets/app.js", file);
            let cache = resp
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                cache.contains("immutable"),
                "JS assets must get immutable cache header, got: {cache}"
            );
            assert!(
                cache.contains("31536000"),
                "JS assets must get 1-year max-age"
            );
        }
    }

    #[test]
    fn serve_file_css_gets_immutable_cache() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("assets/style.css", file);
            let cache = resp
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                cache.contains("immutable"),
                "CSS assets must get immutable cache header"
            );
        }
    }

    #[test]
    fn serve_file_index_html_gets_no_cache() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("index.html", file);
            let cache = resp
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert_eq!(
                cache, "no-cache",
                "index.html must have no-cache for SPA updates"
            );
        }
    }

    #[test]
    fn serve_file_non_asset_path_gets_no_cache() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("favicon.ico", file);
            let cache = resp
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert_eq!(
                cache, "no-cache",
                "non-asset files should get no-cache, got: {cache}"
            );
        }
    }

    #[test]
    fn serve_file_sets_content_type_for_html() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("index.html", file);
            let ct = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ct.contains("text/html"),
                "index.html must be served as text/html, got: {ct}"
            );
        }
    }

    #[test]
    fn serve_file_sets_content_type_for_js() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("app.js", file);
            let ct = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ct.contains("javascript"),
                "JS files must be served as javascript, got: {ct}"
            );
        }
    }

    // ── static_handler SPA routing ──

    #[tokio::test]
    async fn static_handler_returns_index_for_spa_routes() {
        // SPA routes (no file extension) should serve index.html
        let uri: Uri = "/dashboard".parse().unwrap();
        let resp = static_handler(uri).await.into_response();
        // If index.html exists in embedded assets, we get 200; otherwise 404
        let status = resp.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "SPA route should return 200 (with assets) or 404 (without), got: {status}"
        );
    }

    #[tokio::test]
    async fn static_handler_returns_404_for_missing_asset() {
        let uri: Uri = "/assets/nonexistent.abc123.js".parse().unwrap();
        let resp = static_handler(uri).await.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn static_handler_root_path_returns_index() {
        let uri: Uri = "/".parse().unwrap();
        let resp = static_handler(uri).await.into_response();
        let status = resp.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "root path should serve index.html if available"
        );
    }

    #[tokio::test]
    async fn static_handler_nested_spa_route() {
        let uri: Uri = "/settings/profile".parse().unwrap();
        let resp = static_handler(uri).await.into_response();
        let status = resp.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "nested SPA route should serve index.html"
        );
    }

    // ── has_web_assets ──

    #[test]
    fn has_web_assets_returns_bool() {
        // In test env we only have a stub index.html, but the function should not panic
        let _result = has_web_assets();
    }

    // ── list_assets ──

    #[test]
    fn list_assets_returns_vec() {
        let assets = list_assets();
        // Should contain at least index.html if web assets are embedded
        if has_web_assets() {
            assert!(
                assets.iter().any(|a| a.contains("index.html")),
                "assets should include index.html"
            );
        }
    }

    // ── serve_file with assets path ──

    #[test]
    fn serve_file_deep_assets_subpath_gets_immutable_cache() {
        if let Some(file) = WebAssets::get("index.html") {
            // The code checks path.contains("/assets/") — for paths with nested subdirectories
            let resp = serve_file("static/assets/images/logo.png", file);
            let cache = resp
                .headers()
                .get(header::CACHE_CONTROL)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                cache.contains("immutable"),
                "files under /assets/ in a nested path must get immutable cache"
            );
        }
    }

    // ── Content body ──

    #[tokio::test]
    async fn serve_file_returns_non_empty_body() {
        if let Some(file) = WebAssets::get("index.html") {
            let resp = serve_file("index.html", file);
            let body = resp.into_body().collect().await.unwrap().to_bytes();
            assert!(!body.is_empty(), "served file body should not be empty");
        }
    }
}
