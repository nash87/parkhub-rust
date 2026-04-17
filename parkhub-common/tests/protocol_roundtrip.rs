//! Property-based round-trip tests for the request/response envelope DTOs.
//!
//! Focuses on the shapes that cross the HTTP boundary: `ApiError`,
//! `ResponseMeta`, `LoginRequest`, `RegisterRequest`, and the
//! `ApiResponse<T>` wrapper over arbitrary error payloads. The model
//! enums are covered in `property_roundtrip.rs`; this file pins the
//! envelope layer so a drift in `success` / `data` / `error` / `meta`
//! can't slip past the Rust ↔ PHP contract without a test turning red.

use parkhub_common::{ApiError, ApiResponse, LoginRequest, RegisterRequest, ResponseMeta};
use proptest::prelude::*;
use serde_json::Value;

fn arb_small_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_.-]{1,32}".prop_map(String::from)
}

fn arb_api_error() -> impl Strategy<Value = ApiError> {
    (
        arb_small_string(),
        arb_small_string(),
        any::<Option<String>>(),
    )
        .prop_map(|(code, message, details)| ApiError {
            code,
            message,
            details: details.map(Value::String),
        })
}

fn arb_response_meta() -> impl Strategy<Value = ResponseMeta> {
    (
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
        proptest::option::of(any::<i32>()),
    )
        .prop_map(|(page, per_page, total, total_pages)| ResponseMeta {
            page,
            per_page,
            total,
            total_pages,
        })
}

fn arb_login_request() -> impl Strategy<Value = LoginRequest> {
    (arb_small_string(), arb_small_string())
        .prop_map(|(username, password)| LoginRequest { username, password })
}

fn arb_register_request() -> impl Strategy<Value = RegisterRequest> {
    (
        arb_small_string(),
        arb_small_string(),
        arb_small_string(),
        arb_small_string(),
    )
        .prop_map(
            |(email, password, password_confirmation, name)| RegisterRequest {
                email,
                password,
                password_confirmation,
                name,
            },
        )
}

proptest! {
    #[test]
    fn api_error_roundtrips(err in arb_api_error()) {
        let json = serde_json::to_string(&err).unwrap();
        let decoded: ApiError = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(err.code, decoded.code);
        prop_assert_eq!(err.message, decoded.message);
        prop_assert_eq!(err.details, decoded.details);
    }

    #[test]
    fn response_meta_roundtrips(meta in arb_response_meta()) {
        let json = serde_json::to_string(&meta).unwrap();
        let decoded: ResponseMeta = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(meta.page, decoded.page);
        prop_assert_eq!(meta.per_page, decoded.per_page);
        prop_assert_eq!(meta.total, decoded.total);
        prop_assert_eq!(meta.total_pages, decoded.total_pages);
    }

    #[test]
    fn login_request_roundtrips(req in arb_login_request()) {
        let json = serde_json::to_string(&req).unwrap();
        let decoded: LoginRequest = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(req.username, decoded.username);
        prop_assert_eq!(req.password, decoded.password);
    }

    #[test]
    fn register_request_roundtrips(req in arb_register_request()) {
        let json = serde_json::to_string(&req).unwrap();
        let decoded: RegisterRequest = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(req.email, decoded.email);
        prop_assert_eq!(req.password, decoded.password);
        prop_assert_eq!(req.password_confirmation, decoded.password_confirmation);
        prop_assert_eq!(req.name, decoded.name);
    }

    /// `ApiResponse::error(code, message)` must round-trip with
    /// `success=false` and an empty `data` so clients can reliably
    /// branch on `success` rather than sniff for nulls.
    #[test]
    fn api_response_error_keeps_shape(code in arb_small_string(), msg in arb_small_string()) {
        let resp: ApiResponse<()> = ApiResponse::error(code.clone(), msg.clone());
        prop_assert!(!resp.success);
        prop_assert!(resp.data.is_none());
        let err = resp.error.unwrap();
        prop_assert_eq!(err.code, code);
        prop_assert_eq!(err.message, msg);

        // And the shape survives a JSON roundtrip.
        let resp2: ApiResponse<()> = ApiResponse::error("x", "y");
        let json = serde_json::to_string(&resp2).unwrap();
        let decoded: ApiResponse<()> = serde_json::from_str(&json).unwrap();
        prop_assert!(!decoded.success);
        prop_assert!(decoded.data.is_none());
        prop_assert!(decoded.error.is_some());
    }
}
