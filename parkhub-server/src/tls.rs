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

    // Restrict private key file permissions to owner-only (0600)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .context("Failed to set private key file permissions to 0600")?;
    }

    tracing::info!("TLS certificates saved to {}", data_dir.display());

    axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .context("Failed to load generated TLS certificates")
}

/// Generate a self-signed certificate
fn generate_self_signed_cert() -> Result<(String, String)> {
    // Get hostname for certificate
    let hostname = hostname::get().map_or_else(
        |_| "localhost".to_string(),
        |h| h.to_string_lossy().to_string(),
    );

    // Subject alternative names
    let subject_alt_names = vec![hostname, "localhost".to_string(), "127.0.0.1".to_string()];

    // Generate certificate
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
        .context("Failed to generate self-signed certificate")?;

    Ok((cert.pem(), signing_key.serialize_pem()))
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
        write!(fingerprint, "{byte:02X}").unwrap();
    }

    fingerprint
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic() {
        let data = b"test certificate data";
        let fp1 = certificate_fingerprint(data);
        let fp2 = certificate_fingerprint(data);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_format_is_colon_separated_hex() {
        let fp = certificate_fingerprint(b"some bytes");
        // SHA256 produces 32 bytes → 32 hex pairs separated by colons
        let parts: Vec<&str> = fp.split(':').collect();
        assert_eq!(
            parts.len(),
            32,
            "SHA256 fingerprint should have 32 hex pairs"
        );
        for part in &parts {
            assert_eq!(part.len(), 2, "Each hex pair must be 2 chars");
            assert!(
                part.chars().all(|c| c.is_ascii_hexdigit()),
                "Each part must be valid hex: {part}"
            );
        }
    }

    #[test]
    fn fingerprint_uses_uppercase_hex() {
        let fp = certificate_fingerprint(b"uppercase check");
        assert!(
            fp.chars()
                .all(|c| c == ':' || c.is_ascii_uppercase() || c.is_ascii_digit()),
            "Fingerprint should use uppercase hex: {fp}"
        );
    }

    #[test]
    fn fingerprint_different_inputs_produce_different_outputs() {
        let fp1 = certificate_fingerprint(b"cert A");
        let fp2 = certificate_fingerprint(b"cert B");
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn fingerprint_empty_input() {
        let fp = certificate_fingerprint(b"");
        // SHA256 of empty input is well-defined
        let parts: Vec<&str> = fp.split(':').collect();
        assert_eq!(parts.len(), 32);
    }

    #[test]
    fn fingerprint_known_value() {
        // SHA256 of empty bytes is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let fp = certificate_fingerprint(b"");
        assert_eq!(
            fp,
            "E3:B0:C4:42:98:FC:1C:14:9A:FB:F4:C8:99:6F:B9:24:27:AE:41:E4:64:9B:93:4C:A4:95:99:1B:78:52:B8:55"
        );
    }
}
