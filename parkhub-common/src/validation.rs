//! Pure input-validation helpers shared across the ParkHub workspace.
//!
//! These functions are intentionally dependency-free (no `regex`, no
//! `validator`, no runtime state). They encode the "structural" half of
//! the ParkHub input contract — shape and range checks that every layer
//! (server handlers, client SDKs, future CLI tools) should agree on.
//!
//! The heavier, policy-laden validators (password strength rules,
//! server-specific regexes, axum extractors) still live in
//! `parkhub-server::validation`. This module is the seed for extracting
//! the shared subset without forcing every consumer to depend on axum
//! or the `validator` crate.
//!
//! Every function here is pure, total, and safe to fuzz: see
//! `parkhub-common/tests/validation_properties.rs` for the proptest
//! coverage.

use chrono::{DateTime, Utc};

// ───────────────────────────────────────────────────────────────────────────
// Email
// ───────────────────────────────────────────────────────────────────────────

/// Returns `true` if `candidate` is a syntactically plausible email
/// address.
///
/// This is a **structural** check, not an RFC-5322 parser. It enforces:
///
/// * exactly one `@`;
/// * a non-empty local part made of printable ASCII without control
///   characters, spaces, or NUL bytes;
/// * a domain part with at least one `.`;
/// * a final TLD label of at least two ASCII-letter characters;
/// * overall length ≤ 254 bytes (RFC 5321 practical limit).
///
/// The `parkhub-server` layer layers the full policy regex on top of
/// this; fuzz/property tests share this structural base.
#[must_use]
pub fn is_valid_email(candidate: &str) -> bool {
    if candidate.is_empty() || candidate.len() > 254 {
        return false;
    }
    // Reject control chars, NUL bytes, whitespace anywhere — these must
    // never appear in a well-formed email address.
    if candidate
        .chars()
        .any(|c| c.is_control() || c.is_whitespace())
    {
        return false;
    }

    let Some((local, domain)) = candidate.split_once('@') else {
        return false;
    };
    // Exactly one '@'.
    if domain.contains('@') || local.is_empty() || domain.is_empty() {
        return false;
    }

    // Local part: printable ASCII, no leading/trailing dot, no ".." run.
    if !local.is_ascii() || local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    if !local
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || ".!#$%&'*+-/=?^_`{|}~".contains(c))
    {
        return false;
    }

    // Domain: at least one dot, labels 1..=63 chars, TLD ≥ 2 letters.
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return false;
        }
        if label.starts_with('-') || label.ends_with('-') {
            return false;
        }
    }
    let tld = labels[labels.len() - 1];
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    true
}

// ───────────────────────────────────────────────────────────────────────────
// Phone (E.164)
// ───────────────────────────────────────────────────────────────────────────

/// Returns `true` if `candidate` is an E.164 international phone number.
///
/// Accepts `+` followed by 8–15 decimal digits, with no separators.
/// The leading digit after `+` must not be zero (country codes never
/// start with 0 in E.164).
#[must_use]
pub fn is_valid_e164_phone(candidate: &str) -> bool {
    let Some(rest) = candidate.strip_prefix('+') else {
        return false;
    };
    if rest.len() < 8 || rest.len() > 15 {
        return false;
    }
    let mut chars = rest.chars();
    match chars.next() {
        Some(first) if first.is_ascii_digit() && first != '0' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_digit())
}

// ───────────────────────────────────────────────────────────────────────────
// Booking duration
// ───────────────────────────────────────────────────────────────────────────

/// Minimum booking length in minutes (mirrors the server-side rule).
pub const MIN_BOOKING_MINUTES: i32 = 15;

/// Maximum booking length in minutes (24 hours).
pub const MAX_BOOKING_MINUTES: i32 = 24 * 60;

/// Returns `true` when `minutes` is a valid booking duration.
///
/// A valid duration lies in `[MIN_BOOKING_MINUTES, MAX_BOOKING_MINUTES]`.
#[must_use]
pub const fn is_valid_booking_duration(minutes: i32) -> bool {
    minutes >= MIN_BOOKING_MINUTES && minutes <= MAX_BOOKING_MINUTES
}

// ───────────────────────────────────────────────────────────────────────────
// Time range
// ───────────────────────────────────────────────────────────────────────────

/// Half-open time range `[start, end)`.
///
/// Used to normalise booking / operating-hours / availability windows
/// across the workspace without every caller hand-rolling overlap
/// logic. All methods are pure and total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Build a new range. Returns `None` if `start >= end`.
    #[must_use]
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Option<Self> {
        if start < end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Returns `true` iff `start < end`. Always `true` for values built
    /// via [`TimeRange::new`]; provided as a defensive check for
    /// ranges constructed via the public fields.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.start < self.end
    }

    /// Duration of the range. For a range constructed via `new` this
    /// is always strictly positive.
    #[must_use]
    pub fn duration(&self) -> chrono::Duration {
        self.end - self.start
    }

    /// Returns `true` when `self` and `other` share at least one
    /// instant. Symmetric: `a.overlaps(b) == b.overlaps(a)` for any
    /// two valid ranges.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns `true` if `instant` lies in `[start, end)`.
    #[must_use]
    pub fn contains(&self, instant: DateTime<Utc>) -> bool {
        self.start <= instant && instant < self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn email_accepts_basic_cases() {
        assert!(is_valid_email("a@b.co"));
        assert!(is_valid_email("user.name+tag@example.com"));
        assert!(is_valid_email("first.last@sub.domain.co.uk"));
    }

    #[test]
    fn email_rejects_garbage() {
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("nodomain"));
        assert!(!is_valid_email("@nodomain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@domain")); // no dot
        assert!(!is_valid_email("user@.com"));
        assert!(!is_valid_email("user@domain.c")); // TLD too short
        assert!(!is_valid_email("u ser@domain.com")); // space
        assert!(!is_valid_email("user@domain..com")); // empty label
    }

    #[test]
    fn email_rejects_nul_and_control_chars() {
        assert!(!is_valid_email("user\0@domain.com"));
        assert!(!is_valid_email("user@\0.com"));
        assert!(!is_valid_email("user\n@domain.com"));
    }

    #[test]
    fn phone_accepts_e164() {
        assert!(is_valid_e164_phone("+14155552671"));
        assert!(is_valid_e164_phone("+493012345678"));
        assert!(is_valid_e164_phone("+12345678")); // minimum 8 digits
    }

    #[test]
    fn phone_rejects_bad_shape() {
        assert!(!is_valid_e164_phone(""));
        assert!(!is_valid_e164_phone("14155552671")); // no +
        assert!(!is_valid_e164_phone("+0123456789")); // leading 0
        assert!(!is_valid_e164_phone("+123")); // too short
        assert!(!is_valid_e164_phone("+1234567890123456")); // 16 digits
        assert!(!is_valid_e164_phone("+1-415-555-2671")); // separators
        assert!(!is_valid_e164_phone("+141 5552671"));
    }

    #[test]
    fn booking_duration_boundaries() {
        assert!(!is_valid_booking_duration(0));
        assert!(!is_valid_booking_duration(14));
        assert!(is_valid_booking_duration(15));
        assert!(is_valid_booking_duration(24 * 60));
        assert!(!is_valid_booking_duration(24 * 60 + 1));
        assert!(!is_valid_booking_duration(-1));
    }

    #[test]
    fn time_range_new_enforces_strict_order() {
        let a = Utc.with_ymd_and_hms(2026, 4, 17, 10, 0, 0).unwrap();
        let b = Utc.with_ymd_and_hms(2026, 4, 17, 11, 0, 0).unwrap();
        assert!(TimeRange::new(a, b).is_some());
        assert!(TimeRange::new(a, a).is_none());
        assert!(TimeRange::new(b, a).is_none());
    }

    #[test]
    fn time_range_overlap_examples() {
        let t = |h: u32| Utc.with_ymd_and_hms(2026, 4, 17, h, 0, 0).unwrap();
        let r1 = TimeRange::new(t(10), t(12)).unwrap();
        let r2 = TimeRange::new(t(11), t(13)).unwrap();
        let r3 = TimeRange::new(t(12), t(13)).unwrap(); // touches but no overlap
        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));
        assert!(!r1.overlaps(&r3));
        assert!(!r3.overlaps(&r1));
    }
}
