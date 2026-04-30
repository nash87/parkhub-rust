//! Static File Serving
//!
//! Embeds and serves the web frontend from the binary.

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

    // Never hand API paths to the SPA fallback. If an /api/* route reaches
    // this point it means the axum router didn't match it, so the correct
    // answer is 404 (JSON), not index.html with status 200. Returning the
    // SPA HTML would cause API clients to hit a JSON parse error on a 200
    // response, which hides real routing bugs.
    if path.starts_with("api/") || path == "api" {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"success":false,"error":{"code":"NOT_FOUND","message":"API route not found"}}"#,
        )
            .into_response();
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

/// Returns true when the URL path points to a directory that contains
/// content-hashed asset filenames (Astro emits to `_astro/`, Vite + legacy
/// Astro emit to `/assets/`). Files under those paths are safe to mark
/// `immutable` because the URL itself changes on every edit.
///
/// Files outside those directories — `sw.js`, `manifest.json`, `favicon.*`,
/// hand-written root scripts — keep their filename across releases, so they
/// MUST be served `no-cache`. Otherwise a browser that fetched an old `sw.js`
/// will keep serving it for up to a year (the previous max-age was 31536000),
/// pinning users to whatever app version that service worker last cached.
fn is_content_hashed_asset_path(path: &str) -> bool {
    path.starts_with("_astro/")
        || path.contains("/_astro/")
        || path.starts_with("assets/")
        || path.contains("/assets/")
}

fn serve_file(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut response = Response::builder().header(header::CONTENT_TYPE, mime.as_ref());

    if path != "index.html" && is_content_hashed_asset_path(path) {
        response = response.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    } else {
        // Includes index.html and all non-hashed root files (sw.js,
        // manifest.json, favicon.ico, offline.html, …). Always re-validate.
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
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    #[test]
    fn serve_file_css_gets_immutable_cache() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    #[test]
    fn serve_file_index_html_gets_no_cache() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    #[test]
    fn serve_file_non_asset_path_gets_no_cache() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    #[test]
    fn serve_file_sets_content_type_for_html() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    #[test]
    fn serve_file_sets_content_type_for_js() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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
        assert!(
            has_web_assets(),
            "test environment must have embedded web assets — parkhub-web/dist/ wasn't built before parkhub-server compile"
        );
        assert!(
            assets.iter().any(|a| a.contains("index.html")),
            "list_assets() must include index.html"
        );
    }

    // ── serve_file no-cache for non-hashed root assets (regression: sw.js v4.15.0 trap) ──

    #[test]
    fn serve_file_sw_js_must_not_be_immutable() {
        // sw.js is at the dist root and not content-hashed in the filename.
        // Browsers honored the previous `public, max-age=31536000, immutable`
        // for up to 1 year, so users who installed a v4.15.0 sw.js never saw
        // updates to the active service worker — the v4 footer kept rendering
        // long after the deployed binary moved to v5.0.x. sw.js MUST be served
        // with no-cache so the browser re-fetches it on every page load.
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
        let resp = serve_file("sw.js", file);
        let cache = resp
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            cache, "no-cache",
            "sw.js must be served with no-cache so SW updates propagate, got: {cache}"
        );
    }

    #[test]
    fn serve_file_manifest_json_gets_no_cache() {
        // PWA manifest is at the dist root and not hashed; updates to icons,
        // start_url, theme_color etc. should propagate without 1-year staleness.
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
        let resp = serve_file("manifest.json", file);
        let cache = resp
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            cache, "no-cache",
            "manifest.json must be served with no-cache, got: {cache}"
        );
    }

    #[test]
    fn serve_file_root_level_js_not_in_hashed_dir_gets_no_cache() {
        // Any *.js outside /_astro/ or /assets/ is presumed non-hashed and
        // must use no-cache to avoid the same trap as sw.js.
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
        let resp = serve_file("legacy-script.js", file);
        let cache = resp
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            cache, "no-cache",
            "non-hashed root .js files must use no-cache, got: {cache}"
        );
    }

    #[test]
    fn serve_file_astro_hashed_js_gets_immutable_cache() {
        // Files in _astro/ have content hashes (e.g., Welcome.DcWMTKUm.js)
        // so URL changes on every edit — immutable caching is safe.
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
        let resp = serve_file("_astro/Welcome.DcWMTKUm.js", file);
        let cache = resp
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            cache.contains("immutable"),
            "_astro/* files must be immutable-cached (content-hashed URLs), got: {cache}"
        );
    }

    // ── serve_file with assets path ──

    #[test]
    fn serve_file_deep_assets_subpath_gets_immutable_cache() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
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

    // ── Content body ──

    #[tokio::test]
    async fn serve_file_returns_non_empty_body() {
        let file = WebAssets::get("index.html")
            .expect("WebAssets::get(\"index.html\") must succeed: embedded assets missing means parkhub-web/dist/ wasn't built before parkhub-server compile");
        let resp = serve_file("index.html", file);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert!(!body.is_empty(), "served file body should not be empty");
    }
}
