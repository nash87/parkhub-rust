//! AES-256-GCM envelope encryption with PBKDF2 key derivation.
//!
//! The database stores every domain record serialized to JSON, optionally
//! wrapped by AES-256-GCM with a per-database salt. The [`Encryptor`] holds
//! the derived key and performs encrypt/decrypt with fresh nonces.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Result, anyhow};
use pbkdf2::pbkdf2_hmac;
use rand::Rng;
use sha2::Sha256;

/// PBKDF2 iteration count for key derivation.
///
/// 600 000 iterations with HMAC-SHA-256 meets the NIST SP 800-132 (2023)
/// recommendation. This is applied once at database open time, not on every
/// request, so the cost is paid only once per process start.
pub(super) const PBKDF2_ITERATIONS: u32 = 600_000;

pub(super) struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub(super) fn new(passphrase: &str, salt: &[u8]) -> Result<Self> {
        let mut key = [0u8; 32];
        pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
        let cipher =
            Aes256Gcm::new_from_slice(&key).map_err(|e| anyhow!("Failed to create cipher: {e}"))?;
        Ok(Self { cipher })
    }

    pub(super) fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow!("Encryption failed: {e}"))?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    pub(super) fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data: too short"));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow!("Decryption failed: {e}"))
    }
}
