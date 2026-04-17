//! Fuzz target: JWT decode path.
//!
//! Feeds arbitrary bytes into the exact `jsonwebtoken::decode` configuration
//! used by `parkhub_server::jwt::JwtManager::validate_token`. Any input must
//! either parse cleanly or return a typed error — never panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use parkhub_server::fuzz_api::decode_jwt_for_fuzz;

const FUZZ_SECRET: &[u8] = b"fuzz-secret-do-not-use-in-production";

fuzz_target!(|data: &[u8]| {
    let Ok(token) = std::str::from_utf8(data) else {
        return;
    };
    let _ = decode_jwt_for_fuzz(token, FUZZ_SECRET);
});
