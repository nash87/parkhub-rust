//! Multi-country VAT profile configuration layer.
//!
//! ParkHub is shipped to customers in ten locales, so the pricing pipeline
//! cannot assume the German 19 % standard rate. This module provides a thin,
//! declarative table of country tax profiles plus a small resolver that the
//! booking/invoice code paths consume instead of a hard-coded constant.
//!
//! # Scope (deliberately minimal)
//!
//! This is a *configuration layer*, not a full tax engine:
//! * No historical rate tables — the rate in force today is the only one
//!   stored; historical invoices keep whatever rate they were originally
//!   issued with because the persisted `tax_amount` is authoritative.
//! * No per-state US sales tax — the US profile ships a single nominal 0 %
//!   federal rate; US operators still need a local tax-calculation partner.
//! * No EU-OSS one-stop-shop routing — operators supplying consumers
//!   cross-border inside the EU must configure their own profile.
//!
//! # Reverse-charge (Art. 194 VAT Directive)
//!
//! When the buyer is a VAT-registered business in a *different* EU member
//! state than the seller, EU law shifts the VAT liability onto the buyer.
//! The seller then issues a zero-rated invoice bearing a mandatory note.
//! [`resolve_rate`] returns [`ResolvedRate::ReverseCharge`] in that case so
//! the invoice renderer can emit the correct rate + note.
//!
//! # Legal disclaimer
//!
//! Rates current as of 2026-04; consult a tax advisor for production use.
//! The rates below are the standard statutory rates published by each
//! jurisdiction at the time of writing and are exposed purely as sensible
//! defaults. Operators remain responsible for verifying the rate in force
//! for their business and keeping it current in the admin settings store.

use std::fmt;

/// Declarative tax profile for a single country.
///
/// Kept deliberately small — anything more structured belongs in a full
/// tax engine (out of scope for this configuration layer, see module docs).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaxProfile {
    /// ISO 3166-1 alpha-2 country code, uppercase (e.g. `"DE"`, `"AT"`).
    pub country: &'static str,
    /// Standard VAT rate as a fraction (e.g. `0.19` for 19 %).
    pub standard_rate: f64,
    /// Reduced VAT rate, if the jurisdiction publishes one.
    ///
    /// Kept for completeness of the schema even though the current call
    /// sites only consume `standard_rate` — booking/invoice flows use the
    /// standard rate throughout.
    pub reduced_rate: Option<f64>,
    /// Whether this country participates in the EU B2B reverse-charge
    /// regime. Non-EU countries (CH, UK, US) are `false`.
    pub reverse_charge_eu: bool,
}

/// Profiles for the ten representative jurisdictions the product ships to,
/// one per UI locale.
///
/// Ordering is alphabetical by country code for easy scanning; the lookup
/// helpers do not depend on order.
const PROFILES: &[TaxProfile] = &[
    // Austria — Umsatzsteuer, § 10 UStG
    TaxProfile {
        country: "AT",
        standard_rate: 0.20,
        reduced_rate: Some(0.10),
        reverse_charge_eu: true,
    },
    // Switzerland — MWSTG Art. 25 (non-EU; MFN partners use normal rates)
    TaxProfile {
        country: "CH",
        standard_rate: 0.077,
        reduced_rate: Some(0.026),
        reverse_charge_eu: false,
    },
    // Germany — Umsatzsteuergesetz § 12 Abs. 1
    TaxProfile {
        country: "DE",
        standard_rate: 0.19,
        reduced_rate: Some(0.07),
        reverse_charge_eu: true,
    },
    // Spain — IVA, Ley 37/1992
    TaxProfile {
        country: "ES",
        standard_rate: 0.21,
        reduced_rate: Some(0.10),
        reverse_charge_eu: true,
    },
    // France — TVA, CGI art. 278
    TaxProfile {
        country: "FR",
        standard_rate: 0.20,
        reduced_rate: Some(0.055),
        reverse_charge_eu: true,
    },
    // Italy — IVA, DPR 633/1972
    TaxProfile {
        country: "IT",
        standard_rate: 0.22,
        reduced_rate: Some(0.10),
        reverse_charge_eu: true,
    },
    // Netherlands — BTW, Wet op de omzetbelasting 1968
    TaxProfile {
        country: "NL",
        standard_rate: 0.21,
        reduced_rate: Some(0.09),
        reverse_charge_eu: true,
    },
    // Poland — VAT, Ustawa o podatku od towarów i usług
    TaxProfile {
        country: "PL",
        standard_rate: 0.23,
        reduced_rate: Some(0.08),
        reverse_charge_eu: true,
    },
    // United Kingdom — VAT Act 1994 (post-Brexit, non-EU)
    TaxProfile {
        country: "GB",
        standard_rate: 0.20,
        reduced_rate: Some(0.05),
        reverse_charge_eu: false,
    },
    // United States — no federal VAT; state sales tax is out of scope
    TaxProfile {
        country: "US",
        standard_rate: 0.0,
        reduced_rate: None,
        reverse_charge_eu: false,
    },
];

/// Canonical fallback country code used when no default is configured.
pub const DEFAULT_COUNTRY: &str = "DE";

/// Look up a profile by ISO 3166-1 alpha-2 code (case-insensitive).
///
/// Returns `None` for unknown codes; callers typically fall through to
/// [`resolve_profile`] which applies the configured default.
#[must_use]
pub fn profile_for(country: &str) -> Option<&'static TaxProfile> {
    if country.len() != 2 {
        return None;
    }
    let upper = country.to_ascii_uppercase();
    PROFILES.iter().find(|p| p.country == upper)
}

/// Resolve a profile by country code, falling back to [`DEFAULT_COUNTRY`]
/// when the code is unknown or empty.
#[must_use]
pub fn resolve_profile(country: &str) -> &'static TaxProfile {
    profile_for(country).unwrap_or_else(|| {
        profile_for(DEFAULT_COUNTRY).expect("DEFAULT_COUNTRY must always resolve to a profile")
    })
}

/// Every profile currently shipped. Exposed for admin UIs and tests.
#[must_use]
#[allow(dead_code)]
pub fn all_profiles() -> &'static [TaxProfile] {
    PROFILES
}

/// Outcome of resolving a rate for a specific invoice context.
///
/// `Standard` carries the numeric rate to apply; `ReverseCharge` signals the
/// EU B2B zero-rating case so the renderer can emit the Art. 194 note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedRate {
    /// Apply the numeric rate supplied.
    Standard(f64),
    /// Zero-rate the invoice and include the Art. 194 VAT Directive note.
    ReverseCharge,
}

impl ResolvedRate {
    /// Numeric rate as a multiplier (0.0 for reverse-charge).
    #[must_use]
    pub fn as_rate(self) -> f64 {
        match self {
            Self::Standard(r) => r,
            Self::ReverseCharge => 0.0,
        }
    }

    /// True when this resolution is an EU B2B reverse-charge.
    #[must_use]
    pub fn is_reverse_charge(self) -> bool {
        matches!(self, Self::ReverseCharge)
    }
}

impl fmt::Display for ResolvedRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Standard(r) => write!(f, "{:.2}%", r * 100.0),
            Self::ReverseCharge => f.write_str("0% (reverse charge)"),
        }
    }
}

/// Invoice note emitted when [`ResolvedRate::ReverseCharge`] applies.
pub const REVERSE_CHARGE_NOTE: &str = "Reverse charge per Art. 194 VAT Directive";

/// Decide whether EU B2B reverse-charge applies for a given seller/buyer
/// pair.
///
/// All three preconditions must hold:
/// 1. The *buyer* has a verified VAT ID (non-empty after trimming).
/// 2. The seller country's profile is in the EU reverse-charge regime.
/// 3. Seller and buyer countries are different EU member states (also in
///    the reverse-charge regime).
///
/// Same-country sales stay on the seller's standard rate; non-EU buyers
/// fall back to the seller's rate because the Directive does not apply.
#[must_use]
pub fn reverse_charge_applies(
    seller_country: &str,
    buyer_country: &str,
    buyer_vat_id: Option<&str>,
) -> bool {
    let vat_id = match buyer_vat_id {
        Some(v) if !v.trim().is_empty() => v.trim(),
        _ => return false,
    };
    // Minimum plausibility: EU VAT IDs are "XX" + ≥2 alphanumerics. We do
    // *not* perform a VIES round-trip here — that belongs in a full tax
    // engine. Configuration-layer validation is format-only.
    if vat_id.len() < 4 {
        return false;
    }

    let seller = match profile_for(seller_country) {
        Some(p) => p,
        None => return false,
    };
    let buyer = match profile_for(buyer_country) {
        Some(p) => p,
        None => return false,
    };

    if seller.country == buyer.country {
        return false;
    }
    seller.reverse_charge_eu && buyer.reverse_charge_eu
}

/// Full resolver: given seller country, buyer country and buyer VAT ID,
/// return the rate to apply to this invoice.
///
/// * Unknown seller → falls back to the [`DEFAULT_COUNTRY`] profile.
/// * EU B2B with valid buyer VAT ID in a different EU country →
///   [`ResolvedRate::ReverseCharge`].
/// * Everything else → [`ResolvedRate::Standard`] with the seller's
///   standard rate.
#[must_use]
pub fn resolve_rate(
    seller_country: &str,
    buyer_country: &str,
    buyer_vat_id: Option<&str>,
) -> ResolvedRate {
    if reverse_charge_applies(seller_country, buyer_country, buyer_vat_id) {
        return ResolvedRate::ReverseCharge;
    }
    ResolvedRate::Standard(resolve_profile(seller_country).standard_rate)
}

/// Read the seller country for this deployment from the admin settings
/// store, mirroring the precedence used by the invoice renderer.
///
/// Separate from [`resolve_rate`] so booking-creation flows — which do not
/// yet resolve a buyer profile — can still honour the multi-country
/// configuration. Unknown / missing codes resolve to [`DEFAULT_COUNTRY`].
pub async fn resolve_seller_country_from_settings(state: &crate::AppState) -> String {
    for key in ["tax_seller_country", "impressum_country"] {
        if let Ok(Some(v)) = state.db.get_setting(key).await {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    if let Ok(v) = std::env::var("PARKHUB_TAX_COUNTRY") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    DEFAULT_COUNTRY.to_string()
}

/// Resolve the seller's standard VAT rate from the admin settings store.
///
/// Used by booking-creation flows that don't yet know the buyer's country
/// or VAT ID — those details are resolved later at invoice-render time
/// (see `api::invoices::get_booking_invoice_pdf`) where reverse-charge may
/// additionally kick in. Because the persisted `tax` on a booking is the
/// rate in force *when the booking was created*, this helper is what keeps
/// `booking.pricing.tax` consistent with the operator's configured rate.
pub async fn resolve_standard_rate(state: &crate::AppState) -> f64 {
    let country = resolve_seller_country_from_settings(state).await;
    resolve_profile(&country).standard_rate
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_profile_de_rate_19() {
        let de = resolve_profile("DE");
        assert_eq!(de.country, "DE");
        assert!((de.standard_rate - 0.19).abs() < f64::EPSILON);
        assert_eq!(de.reduced_rate, Some(0.07));
        assert!(de.reverse_charge_eu);
    }

    #[test]
    fn test_tax_profile_ch_rate_77() {
        let ch = resolve_profile("CH");
        assert_eq!(ch.country, "CH");
        // Switzerland's standard rate: 7.7 %.
        assert!((ch.standard_rate - 0.077).abs() < f64::EPSILON);
        // Non-EU → no reverse-charge regime.
        assert!(!ch.reverse_charge_eu);
    }

    #[test]
    fn test_reverse_charge_eu_b2b_applies_0_percent() {
        // German seller, Austrian B2B buyer with valid VAT ID → 0 %.
        let resolved = resolve_rate("DE", "AT", Some("ATU12345678"));
        assert_eq!(resolved, ResolvedRate::ReverseCharge);
        assert!((resolved.as_rate() - 0.0).abs() < f64::EPSILON);
        assert!(resolved.is_reverse_charge());
    }

    #[test]
    fn test_reverse_charge_same_country_does_not_apply() {
        // German seller, German buyer with a VAT ID → domestic sale,
        // standard 19 % rate applies.
        let resolved = resolve_rate("DE", "DE", Some("DE123456789"));
        match resolved {
            ResolvedRate::Standard(r) => {
                assert!((r - 0.19).abs() < f64::EPSILON);
            }
            ResolvedRate::ReverseCharge => {
                panic!("same-country B2B must not trigger reverse charge");
            }
        }
        assert!(!resolved.is_reverse_charge());
    }

    #[test]
    fn test_resolve_profile_unknown_falls_back_to_default() {
        let p = resolve_profile("ZZ");
        assert_eq!(p.country, DEFAULT_COUNTRY);
    }

    #[test]
    fn test_resolve_profile_case_insensitive() {
        assert_eq!(resolve_profile("de").country, "DE");
        assert_eq!(resolve_profile("gB").country, "GB");
    }

    #[test]
    fn test_ten_profiles_shipped() {
        // Exactly matches the ten UI locales the product ships.
        assert_eq!(all_profiles().len(), 10);
    }

    #[test]
    fn test_reverse_charge_requires_vat_id() {
        // Different EU country but no VAT ID → not reverse-charge.
        assert!(!reverse_charge_applies("DE", "FR", None));
        assert!(!reverse_charge_applies("DE", "FR", Some("")));
        assert!(!reverse_charge_applies("DE", "FR", Some("   ")));
        // VAT ID too short to be plausible.
        assert!(!reverse_charge_applies("DE", "FR", Some("FR1")));
    }

    #[test]
    fn test_reverse_charge_non_eu_buyer() {
        // Swiss buyer with VAT ID from a German seller → CH is non-EU,
        // so the Directive does not apply; standard DE rate is charged.
        let resolved = resolve_rate("DE", "CH", Some("CHE123456789"));
        match resolved {
            ResolvedRate::Standard(r) => {
                assert!((r - 0.19).abs() < f64::EPSILON);
            }
            ResolvedRate::ReverseCharge => {
                panic!("non-EU buyer must not trigger EU reverse-charge");
            }
        }
    }

    #[test]
    fn test_resolved_rate_display() {
        assert_eq!(format!("{}", ResolvedRate::Standard(0.19)), "19.00%");
        assert_eq!(
            format!("{}", ResolvedRate::ReverseCharge),
            "0% (reverse charge)"
        );
    }
}
