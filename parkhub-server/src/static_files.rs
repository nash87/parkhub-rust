//! Static File Serving
//!
//! Embeds and serves the web frontend from the binary.

use axum::{
    body::Body,
    http::{header, Request, StatusCode, Uri},
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
    if !path.contains('.') || path.is_empty() {
        if let Some(file) = WebAssets::get("index.html") {
            return serve_file("index.html", file);
        }
    }
    
    // 404 for missing assets
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

fn serve_file(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref());
    
    // Add cache headers for assets (not index.html)
    if path != "index.html" && (path.contains("/assets/") || path.ends_with(".js") || path.ends_with(".css")) {
        response = response
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    } else {
        response = response
            .header(header::CACHE_CONTROL, "no-cache");
    }
    
    response
        .body(Body::from(file.data.into_owned()))
        .unwrap()
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
