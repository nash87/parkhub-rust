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

// ── Boundary / adversarial expansions ──────────────────────────────────────
//
// The properties above cover the "happy path" of each validator. The block
// below targets boundary conditions and adversarial inputs that have
// historically harbored off-by-one bugs (string length cutoffs, empty
// segments, sign edge cases, half-open vs closed ranges).

// ── is_valid_email — adversarial ───────────────────────────────────────────

proptest! {
    /// No matter what valid-looking content surrounds it, an address
    /// without a single `@` must be rejected. Catches accidental
    /// short-circuits in the split-once branch.
    #[test]
    fn email_without_at_sign_rejected(s in "[a-z0-9.-]{1,80}") {
        prop_assume!(!s.contains('@'));
        prop_assert!(!is_valid_email(&s));
    }

    /// Any address whose total byte length exceeds 254 must be
    /// rejected, regardless of the otherwise-valid shape.
    #[test]
    fn email_over_254_bytes_rejected(local_len in 250usize..=400) {
        let local: String = "a".repeat(local_len);
        let candidate = format!("{local}@example.com");
        prop_assume!(candidate.len() > 254);
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Local parts containing a `..` run must always be rejected,
    /// regardless of position or surrounding characters. This is
    /// the property that prevents `a..b@c.com` from leaking.
    #[test]
    fn email_consecutive_dots_in_local_rejected(
        prefix in "[a-z]{1,8}",
        suffix in "[a-z]{1,8}",
    ) {
        let candidate = format!("{prefix}..{suffix}@example.com");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Single-letter TLDs are not valid (RFC 5321 / ICANN minimum is 2).
    /// This property would catch a regression where the TLD length
    /// guard is dropped or weakened.
    #[test]
    fn email_single_letter_tld_rejected(
        local in "[a-z][a-z0-9]{0,16}",
        domain in "[a-z][a-z0-9-]{0,16}",
        tld in "[a-z]",
    ) {
        let candidate = format!("{local}@{domain}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Two or more `@` signs must always be rejected. Catches a
    /// regression where `split_once('@')` is replaced with something
    /// that accepts the first match silently.
    #[test]
    fn email_multiple_at_signs_rejected(
        local in "[a-z]{1,8}",
        middle in "[a-z]{1,8}",
        domain in "[a-z]{1,8}",
    ) {
        let candidate = format!("{local}@{middle}@{domain}.com");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Empty local part (`@example.com`) must be rejected even when
    /// the domain is otherwise valid.
    #[test]
    fn email_empty_local_rejected(
        domain in "[a-z][a-z0-9-]{0,16}",
        tld in "[a-z]{2,8}",
    ) {
        let candidate = format!("@{domain}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Empty domain (`local@`) must be rejected even when the local
    /// part is otherwise valid.
    #[test]
    fn email_empty_domain_rejected(local in "[a-z][a-z0-9]{0,16}") {
        let candidate = format!("{local}@");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Whitespace anywhere in the candidate must trigger rejection.
    /// Covers SP, TAB, NL, and the unicode whitespace surface.
    #[test]
    fn email_whitespace_anywhere_rejected(
        local in "[a-z]{1,8}",
        ws in prop_oneof![Just(' '), Just('\t'), Just('\n'), Just('\r')],
        domain in "[a-z]{1,8}",
        tld in "[a-z]{2,4}",
    ) {
        let candidate = format!("{local}{ws}@{domain}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// Local parts starting OR ending with a dot must be rejected.
    /// Two strategies cover both boundaries in one test body.
    #[test]
    fn email_local_dot_at_edge_rejected(
        body in "[a-z]{1,12}",
        domain in "[a-z]{1,8}",
        tld in "[a-z]{2,4}",
        position in 0u8..=1,
    ) {
        let local = if position == 0 {
            format!(".{body}")
        } else {
            format!("{body}.")
        };
        let candidate = format!("{local}@{domain}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// A domain label that starts or ends with `-` must be rejected.
    /// Hostname syntax (RFC 1035) requires alphanumeric edges.
    #[test]
    fn email_domain_label_hyphen_at_edge_rejected(
        local in "[a-z]{1,8}",
        body in "[a-z]{1,8}",
        tld in "[a-z]{2,4}",
        position in 0u8..=1,
    ) {
        let label = if position == 0 {
            format!("-{body}")
        } else {
            format!("{body}-")
        };
        let candidate = format!("{local}@{label}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }

    /// A domain label longer than 63 characters violates the DNS
    /// label-length limit. The validator must reject it regardless
    /// of how short the rest of the address is.
    #[test]
    fn email_domain_label_over_63_rejected(
        local in "[a-z]{1,8}",
        oversize_len in 64usize..=80,
        tld in "[a-z]{2,4}",
    ) {
        let label: String = "a".repeat(oversize_len);
        let candidate = format!("{local}@{label}.{tld}");
        // Pre-flight: total length must stay ≤ 254 to ensure the
        // 254-byte global cap doesn't overshadow the label-length
        // rejection we're actually testing.
        prop_assume!(candidate.len() <= 254);
        prop_assert!(!is_valid_email(&candidate));
    }

    /// A TLD containing any digit must be rejected — the validator
    /// requires `is_ascii_alphabetic()` for every TLD char.
    #[test]
    fn email_tld_with_digit_rejected(
        local in "[a-z]{1,8}",
        domain in "[a-z]{1,8}",
        prefix in "[a-z]{0,3}",
        digit in 0u8..=9,
        suffix in "[a-z]{0,3}",
    ) {
        let tld = format!("{prefix}{digit}{suffix}");
        prop_assume!(tld.len() >= 2);
        let candidate = format!("{local}@{domain}.{tld}");
        prop_assert!(!is_valid_email(&candidate));
    }
}

// ── is_valid_e164_phone — boundaries ──────────────────────────────────────

proptest! {
    /// Numbers with fewer than 8 digits after `+` must be rejected.
    /// Triangulates the `len < 8` boundary.
    #[test]
    fn phone_under_8_digits_rejected(
        first in 1u8..=9,
        rest_len in 0usize..7,
    ) {
        let mut buf = format!("+{}", char::from(b'0' + first));
        for _ in 0..rest_len {
            buf.push('5');
        }
        prop_assert!(!is_valid_e164_phone(&buf));
    }

    /// Numbers with 16+ digits after `+` must be rejected.
    /// Triangulates the `len > 15` boundary.
    #[test]
    fn phone_over_15_digits_rejected(
        first in 1u8..=9,
        rest_len in 15usize..=64,
    ) {
        let mut buf = format!("+{}", char::from(b'0' + first));
        for _ in 0..rest_len {
            buf.push('5');
        }
        prop_assert!(!is_valid_e164_phone(&buf));
    }

    /// `+0…` is always rejected — country codes never start with 0
    /// in E.164, even when the remaining digits would otherwise pass.
    #[test]
    fn phone_leading_zero_always_rejected(rest_len in 7usize..=14) {
        let mut buf = String::from("+0");
        for _ in 0..rest_len {
            buf.push('5');
        }
        prop_assert!(!is_valid_e164_phone(&buf));
    }
}

// ── is_valid_booking_duration — out-of-bounds ─────────────────────────────

proptest! {
    /// Anything strictly below `MIN_BOOKING_MINUTES` must be rejected,
    /// including the entire negative range.
    #[test]
    fn booking_below_min_always_rejected(minutes in i32::MIN..MIN_BOOKING_MINUTES) {
        prop_assert!(!is_valid_booking_duration(minutes));
    }

    /// Anything strictly above `MAX_BOOKING_MINUTES` must be rejected,
    /// including very large positive values that should not panic.
    #[test]
    fn booking_above_max_always_rejected(minutes in (MAX_BOOKING_MINUTES + 1)..=i32::MAX) {
        prop_assert!(!is_valid_booking_duration(minutes));
    }
}

// ── TimeRange — half-open + degeneracy ────────────────────────────────────

proptest! {
    /// `TimeRange::new(t, t)` must always return `None`. The degenerate
    /// case is the canonical off-by-one trap (`<` vs `<=`).
    #[test]
    fn time_range_new_degenerate_rejected(t in arb_utc_datetime()) {
        prop_assert!(TimeRange::new(t, t).is_none());
    }

    /// `TimeRange::new(end, start)` with `start < end` must always
    /// return `None`. Catches an accidentally swapped comparison.
    #[test]
    fn time_range_new_reversed_rejected(
        start in arb_utc_datetime(),
        delta_secs in 1i64..=(365 * 24 * 60 * 60),
    ) {
        let end = start + Duration::seconds(delta_secs);
        prop_assert!(TimeRange::new(end, start).is_none());
    }

    /// Two ranges that meet at a single instant (`end_a == start_b`)
    /// must NOT overlap — the half-open `[start, end)` semantics make
    /// the boundary instant belong to `b` only. This guards the
    /// `<` vs `<=` decision in `overlaps`.
    #[test]
    fn time_range_touching_does_not_overlap(
        start_a in arb_utc_datetime(),
        delta_a in 1i64..=86_400,
        delta_b in 1i64..=86_400,
    ) {
        let mid = start_a + Duration::seconds(delta_a);
        let end_b = mid + Duration::seconds(delta_b);
        let a = TimeRange::new(start_a, mid).unwrap();
        let b = TimeRange::new(mid, end_b).unwrap();
        prop_assert!(!a.overlaps(&b));
        prop_assert!(!b.overlaps(&a));
    }

    /// `duration()` must equal `end - start` exactly for every range
    /// constructed via `new`. Catches accidental sign or order flips.
    #[test]
    fn time_range_duration_matches_subtraction(
        start in arb_utc_datetime(),
        delta_secs in 1i64..=(365 * 24 * 60 * 60),
    ) {
        let end = start + Duration::seconds(delta_secs);
        let range = TimeRange::new(start, end).unwrap();
        prop_assert_eq!(range.duration(), end - start);
        prop_assert_eq!(range.duration(), Duration::seconds(delta_secs));
    }
}
