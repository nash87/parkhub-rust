//! Property-based tests for the pure validators in
//! `parkhub_common::validation`.
//!
//! These live alongside `property_roundtrip.rs` and `protocol_roundtrip.rs`
//! (same `tests/*.rs` convention) so the whole property suite runs with a
//! single `cargo test -p parkhub-common --tests` invocation. Each
//! `proptest!` block exercises the default 256 cases.
//!
//! The properties encoded here:
//!
//! * `is_valid_email` — local parts shaped like `[a-z0-9._-]{1,32}` always
//!   pass when joined to a fixed `@example.com` domain; strings containing
//!   NUL bytes or ASCII control chars are always rejected, regardless of
//!   where they appear.
//! * `is_valid_e164_phone` — any `+` followed by 8–15 digits with a
//!   non-zero leading digit passes; random human-entered strings with
//!   separators / letters are rejected.
//! * `is_valid_booking_duration` — the bounds match the documented
//!   `[MIN_BOOKING_MINUTES, MAX_BOOKING_MINUTES]` interval exactly.
//! * `TimeRange` — every range produced by `new(start, end)` with
//!   `start < end` is valid, has positive duration, contains its start,
//!   does not contain its end, and is symmetric under `overlaps`
//!   (including `overlaps(self, self) == true`).

use chrono::{DateTime, Duration, TimeZone, Utc};
use parkhub_common::validation::{
    MAX_BOOKING_MINUTES, MIN_BOOKING_MINUTES, TimeRange, is_valid_booking_duration,
    is_valid_e164_phone, is_valid_email,
};
use proptest::prelude::*;

// ── helpers ────────────────────────────────────────────────────────────────

fn arb_utc_datetime() -> impl Strategy<Value = DateTime<Utc>> {
    // Any second within a ~60-year window around the epoch keeps chrono
    // well away from its overflow boundaries.
    (-946_684_800_i64..=1_893_456_000_i64).prop_map(|secs| {
        Utc.timestamp_opt(secs, 0)
            .single()
            .expect("timestamp fits in chrono's DateTime<Utc> range")
    })
}

// ── is_valid_email ─────────────────────────────────────────────────────────

proptest! {
    /// Any well-formed local part joined to a fixed valid domain must be
    /// accepted. This guards against accidental over-rejection as the
    /// local-part character class evolves. The regex deliberately
    /// avoids producing consecutive `.` runs and leading/trailing
    /// dots — those are illegal under RFC 5321 and our checker
    /// rejects them on purpose.
    #[test]
    fn valid_local_part_always_accepted(
        local in "[a-z0-9][a-z0-9_+-]{0,30}[a-z0-9]",
    ) {
        let email = format!("{local}@example.com");
        prop_assert!(
            is_valid_email(&email),
            "expected `{email}` to validate, but is_valid_email returned false",
        );
    }

    /// NUL bytes and ASCII control characters must never appear in a
    /// valid email, no matter where they land. This is the property
    /// that would have caught the `\0` bug if the validator leaked it.
    #[test]
    fn control_chars_always_rejected(
        prefix in "[a-z]{0,16}",
        suffix in "[a-z]{0,16}",
        ctrl in prop_oneof![Just('\0'), Just('\n'), Just('\r'), Just('\t'), Just('\x07')],
    ) {
        let poisoned = format!("{prefix}{ctrl}{suffix}@example.com");
        prop_assert!(
            !is_valid_email(&poisoned),
            "control char {:?} leaked past is_valid_email in `{poisoned}`",
            ctrl,
        );
        let poisoned_domain = format!("user@exa{ctrl}mple.com");
        prop_assert!(
            !is_valid_email(&poisoned_domain),
            "control char {:?} leaked past is_valid_email in `{poisoned_domain}`",
            ctrl,
        );
    }
}

// ── is_valid_e164_phone ────────────────────────────────────────────────────

proptest! {
    /// Any string matching the E.164 shape (`+`, non-zero lead digit,
    /// 8–15 digits total) must validate.
    #[test]
    fn e164_shape_always_accepted(
        first in 1u8..=9,
        rest in prop::collection::vec(0u8..=9, 7..=14),
    ) {
        let mut buf = String::with_capacity(rest.len() + 2);
        buf.push('+');
        buf.push(char::from(b'0' + first));
        for d in &rest {
            buf.push(char::from(b'0' + *d));
        }
        prop_assert!(
            is_valid_e164_phone(&buf),
            "expected `{buf}` to validate as E.164",
        );
    }

    /// Strings containing ASCII letters, spaces, or dashes in the
    /// digit portion must always be rejected — E.164 forbids
    /// punctuation.
    #[test]
    fn non_digit_payload_rejected(
        first in 1u8..=9,
        rest_len in 7usize..=14,
        junk in prop_oneof![Just(' '), Just('-'), Just('.'), Just('('), Just('a'), Just('Z')],
        junk_pos in 0usize..14,
    ) {
        let digits: String = (0..rest_len).map(|_| '5').collect();
        let mut buf = format!("+{}{digits}", char::from(b'0' + first));
        let pos = 2 + (junk_pos % buf.len().saturating_sub(2).max(1));
        buf.insert(pos, junk);
        prop_assert!(
            !is_valid_e164_phone(&buf),
            "junk char {:?} leaked past is_valid_e164_phone in `{buf}`",
            junk,
        );
    }
}

// ── is_valid_booking_duration ──────────────────────────────────────────────

proptest! {
    /// The validator is exactly the closed interval
    /// `[MIN_BOOKING_MINUTES, MAX_BOOKING_MINUTES]`.
    #[test]
    fn booking_duration_matches_bounds(minutes in -1_000_000i32..=1_000_000) {
        let expected = (MIN_BOOKING_MINUTES..=MAX_BOOKING_MINUTES).contains(&minutes);
        prop_assert_eq!(is_valid_booking_duration(minutes), expected);
    }
}

// ── TimeRange ──────────────────────────────────────────────────────────────

proptest! {
    /// Every range built from `start < end` must be valid, have a
    /// strictly positive duration, and contain its start instant but
    /// not its end instant (half-open semantics).
    #[test]
    fn time_range_new_invariants(
        start in arb_utc_datetime(),
        delta_secs in 1i64..=(365 * 24 * 60 * 60),
    ) {
        let end = start + Duration::seconds(delta_secs);
        let range = TimeRange::new(start, end)
            .expect("start < end by construction");
        prop_assert!(range.is_valid());
        prop_assert!(range.duration() > Duration::zero());
        prop_assert!(range.contains(start));
        prop_assert!(!range.contains(end));
    }

    /// `overlaps` must be symmetric and reflexive on any valid
    /// range. The reflexive case would also catch a bug where
    /// `<=` is swapped for `<` on the identical-range path.
    #[test]
    fn time_range_overlap_is_symmetric_and_reflexive(
        start_a in arb_utc_datetime(),
        delta_a in 1i64..=86_400,
        start_b in arb_utc_datetime(),
        delta_b in 1i64..=86_400,
    ) {
        let a = TimeRange::new(start_a, start_a + Duration::seconds(delta_a)).unwrap();
        let b = TimeRange::new(start_b, start_b + Duration::seconds(delta_b)).unwrap();
        prop_assert_eq!(a.overlaps(&b), b.overlaps(&a));
        prop_assert!(a.overlaps(&a));
        prop_assert!(b.overlaps(&b));
    }
}
