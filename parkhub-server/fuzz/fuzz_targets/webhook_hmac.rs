//! Fuzz target: webhook HMAC-SHA256 signature verification.
//!
//! Splits the input into header / body / secret slices and feeds them to the
//! Stripe-style verifier mirrored in `parkhub_server::fuzz_api`. The verifier
//! must return `true`/`false` for any input — never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use parkhub_server::fuzz_api::verify_webhook_hmac_for_fuzz;

fuzz_target!(|data: &[u8]| {
    // Split: first byte = header length (0..=255), next `n` = header,
    // next 1 byte = secret length, remainder split into secret + body.
    if data.len() < 3 {
        return;
    }
    let hdr_len = usize::from(data[0]);
    let rest = &data[1..];
    if hdr_len > rest.len() {
        return;
    }
    let (hdr_bytes, rest) = rest.split_at(hdr_len);
    let Ok(header) = std::str::from_utf8(hdr_bytes) else {
        return;
    };
    if rest.is_empty() {
        return;
    }
    let secret_len = usize::from(rest[0]).min(rest.len() - 1);
    let (secret, body) = rest[1..].split_at(secret_len);
    let _ = verify_webhook_hmac_for_fuzz(header, body, secret);
});
