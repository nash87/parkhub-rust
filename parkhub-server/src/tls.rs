//! TLS Certificate Management
//!
//! Generates and loads self-signed certificates for secure connections.

use anyhow::{Context, Result};
use rcgen::{CertifiedKey, generate_simple_self_signed};
use std::path::Path;
use std::sync::Once;

/// Ensure the Rustls crypto provider is installed (only once)
static CRYPTO_PROVIDER_INIT: Once = Once::new();

fn ensure_crypto_provider() {
    CRYPTO_PROVIDER_INIT.call_once(|| {
        // Install the ring crypto provider for Rustls
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Load existing TLS config or create new self-signed certificate
pub async fn load_or_create_tls_config(
    data_dir: &Path,
) -> Result<axum_server::tls_rustls::RustlsConfig> {
    // Ensure crypto provider is initialized
    ensure_crypto_provider();

    let cert_path = data_dir.join("server.crt");
    let key_path = data_dir.join("server.key");

    // Check if certificates exist
    if cert_path.exists() && key_path.exists() {
        tracing::info!("Loading existing TLS certificates");
        return axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
            .await
            .context("Failed to load TLS certificates");
    }

    // Generate new self-signed certificate
    tracing::info!("Generating new self-signed TLS certificate");
    let (cert_pem, key_pem) = generate_self_signed_cert()?;

    // Save certificates
    std::fs::write(&cert_path, &cert_pem).context("Failed to write certificate")?;
    std::fs::write(&key_path, &key_pem).context("Failed to write private key")?;

    tracing::info!("TLS certificates saved to {}", data_dir.display());

    axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .context("Failed to load generated TLS certificates")
}

/// Generate a self-signed certificate
fn generate_self_signed_cert() -> Result<(String, String)> {
    // Get hostname for certificate
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "localhost".to_string());

    // Subject alternative names
    let subject_alt_names = vec![
        hostname.clone(),
        "localhost".to_string(),
        "127.0.0.1".to_string(),
    ];

    // Generate certificate
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)
        .context("Failed to generate self-signed certificate")?;

    Ok((cert.pem(), key_pair.serialize_pem()))
}

/// Calculate SHA256 fingerprint of a certificate
pub fn certificate_fingerprint(cert_der: &[u8]) -> String {
    use std::fmt::Write;

    let digest = ring::digest::digest(&ring::digest::SHA256, cert_der);
    let mut fingerprint = String::new();

    for (i, byte) in digest.as_ref().iter().enumerate() {
        if i > 0 {
            fingerprint.push(':');
        }
        write!(fingerprint, "{:02X}", byte).unwrap();
    }

    fingerprint
}
