//! Fuzz-only library surface for `parkhub-server`.
//!
//! This crate is a binary (`src/main.rs`) in all production builds. This
//! `lib.rs` exists solely so `cargo-fuzz` harnesses can link against a stable,
//! minimal API that mirrors — line-for-line — the parsing / verification code
//! paths used by the real server (JWT decode in `src/jwt.rs`, Stripe webhook
//! signature verify in `src/api/stripe.rs`).
//!
//! The functions below intentionally duplicate the hot bytes of their real
//! counterparts instead of pulling in the entire module tree. This keeps the
//! fuzz build small (no AppState, DB, Axum router), while still exercising the
//! same third-party parsers (`jsonwebtoken`, `hmac`, `sha2`) with the same
//! configuration the server uses. If the real code ever diverges, these
//! wrappers must be updated to match.
//!
//! Not part of any public API. Do not import from production code.

#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "fuzzing")]
pub mod fuzz_api {
    //! Minimal, dependency-free entry points for `cargo-fuzz` targets.

    use hmac::{Hmac, Mac};
    use jsonwebtoken::{DecodingKey, Validation, decode};
    use serde::{Deserialize, Serialize};
    use sha2::Sha256;
    use subtle::ConstantTimeEq;

    /// Mirror of `parkhub_server::jwt::Claims` for fuzz decoding.
    /// Fields mirror the real struct; only used to satisfy the generic
    /// on `jsonwebtoken::decode::<Claims>(...)`.
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Claims {
        pub sub: String,
        pub username: String,
        pub role: String,
        pub iat: i64,
        pub exp: i64,
        pub iss: String,
        pub token_type: String,
        pub jti: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub family_id: Option<String>,
    }

    /// Fuzz-only wrapper around the exact `jsonwebtoken::decode` call made by
    /// `JwtManager::validate_token` in `src/jwt.rs`.
    ///
    /// Must never panic on arbitrary input. Any decode failure returns `Err`.
    pub fn decode_jwt_for_fuzz(
        token: &str,
        secret: &[u8],
    ) -> Result<Claims, jsonwebtoken::errors::Error> {
        let decoding_key = DecodingKey::from_secret(secret);
        let mut validation = Validation::default();
        validation.set_issuer(&["parkhub"]);
        // Disable expiry check so the fuzzer reaches deeper parse paths
        // instead of short-circuiting on ExpiredSignature.
        validation.validate_exp = false;
        decode::<Claims>(token, &decoding_key, &validation).map(|td| td.claims)
    }

    /// Fuzz-only mirror of `verify_stripe_signature` in `src/api/stripe.rs`.
    ///
    /// Parses a Stripe-style `t=<ts>,v1=<hex>` header, HMAC-SHA256s
    /// `"{ts}.{body}"` with `secret`, and compares in constant time. Returns
    /// `true` iff at least one provided `v1=` signature matches. Must never
    /// panic on arbitrary input.
    ///
    /// The timestamp tolerance check is skipped here — fuzzing targets the
    /// parser and crypto paths, not wall-clock logic.
    #[must_use]
    pub fn verify_webhook_hmac_for_fuzz(sig_header: &str, body: &[u8], secret: &[u8]) -> bool {
        let mut timestamp: Option<&str> = None;
        let mut signatures: Vec<&str> = Vec::new();

        for part in sig_header.split(',') {
            let part = part.trim();
            if let Some(ts) = part.strip_prefix("t=") {
                timestamp = Some(ts);
            } else if let Some(sig) = part.strip_prefix("v1=") {
                signatures.push(sig);
            }
        }

        let Some(timestamp) = timestamp else {
            return false;
        };
        if signatures.is_empty() {
            return false;
        }
        if timestamp.parse::<i64>().is_err() {
            return false;
        }

        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret) else {
            return false;
        };
        mac.update(timestamp.as_bytes());
        mac.update(b".");
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        let expected_bytes = expected.as_bytes();

        signatures
            .iter()
            .any(|sig| expected_bytes.ct_eq(sig.as_bytes()).into())
    }
}
