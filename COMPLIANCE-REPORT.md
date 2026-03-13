# Legal Compliance Audit Report -- ParkHub

**Audit Date:** 2026-03-14
**Scope:** German, EU, and international legal compliance for self-hosted parking management SPA
**Repos Audited:** parkhub-rust, parkhub-php
**Auditor:** Automated compliance review (Claude Code)

---

## Executive Summary

ParkHub demonstrates **strong legal compliance fundamentals** for a self-hosted open-source application. The on-premise architecture eliminates most international data transfer concerns, and the existing legal template suite covers the core German regulatory requirements (DSGVO, DDG, BGB, TTDSG). The project correctly positions all legal documents as **operator templates** rather than binding legal texts, which is the appropriate approach for open-source software.

**Key findings:**
- 7 of 7 required legal templates present and substantively correct
- New UX features (onboarding hints, feature flags) use localStorage in a TTDSG-compliant manner
- Several documentation gaps identified regarding new localStorage keys, service worker caching, and DPIA guidance
- The Rust datenschutz template is missing the Art. 7 Abs. 3 consent withdrawal right (PHP version has it)
- No BFSG (accessibility) documentation exists yet

**Overall Rating: PASS with recommendations**

---

## 1. Impressum Template (DDG SS 5)

**Status: PASS**

| Requirement | Present | Notes |
|---|---|---|
| Provider name and legal form | Yes | Placeholder fields provided |
| Full postal address | Yes | Street, PLZ, city, country |
| Email (direct contact) | Yes | Note correctly states no contact-form-only |
| Phone | Yes | Marked as recommended (correct -- not strictly mandatory but strongly advised) |
| Handelsregister | Yes | Conditional on registration |
| USt-IdNr (SS 27a UStG) | Yes | Conditional on VAT liability |
| Responsible person (SS 18 Abs. 2 MStV) | Yes | For journalistic content |
| Reference to DDG (not TMG) | Yes | Correctly updated to DDG |

**Minor recommendation:** Add a placeholder for Wirtschafts-Identifikationsnummer (W-IdNr), which becomes mandatory under the new identification number law. Not yet enforced for all businesses but forward-looking.

---

## 2. Datenschutzerklarung Template (Art. 13/14 DSGVO)

**Status: WARN -- gaps found, fixes applied**

| Requirement | Present | Notes |
|---|---|---|
| Controller identity | Yes | Placeholder |
| Data categories and purposes | Yes | Sections 1.1-1.4 |
| Legal basis per processing activity | Yes | Art. 6 citations correct |
| Retention periods | Yes | Including SS 147 AO for bookings |
| Third-party disclosure | Yes | SMTP exception documented |
| Data subject rights (Art. 15-22) | Yes | Table format |
| Right to lodge complaint (Art. 77) | Yes | DPA examples provided |
| Right to withdraw consent (Art. 7 Abs. 3) | **Rust: No / PHP: Yes** | **FIXED** -- added to Rust template |
| Technical security measures | Yes | TLS, hashing, encryption |
| localStorage documentation | Partial | Token and theme only -- **FIXED** to include all keys |
| Automated decision-making (Art. 13(2)(f)) | No | Not applicable (no automated decisions) -- acceptable omission |
| International transfer info (Art. 13(1)(f)) | Partial | Covered under "no third parties" but could be explicit |

**Fixes applied:**
1. Added Art. 7 Abs. 3 consent withdrawal right to Rust template (was already in PHP)
2. Expanded Section 5 (Cookies und lokaler Speicher) to document all localStorage keys including `parkhub_hint_*`, `parkhub_features`, `parkhub_usecase`, and `i18next` language preference

---

## 3. AGB Template (SSSS 305-310 BGB)

**Status: PASS**

| Requirement | Present | Notes |
|---|---|---|
| Scope (SS 1) | Yes | B2C and B2B differentiation |
| Contract formation (SS 2) | Yes | Booking confirmation as acceptance |
| Cancellation terms (SS 4) | Yes | Flexible operator configuration |
| Consumer withdrawal (SS 5) | Yes | 14-day period, SS 356 Abs. 4 early expiry |
| Pricing and VAT (SS 6) | Yes | MwSt. reference, SS 288 BGB interest |
| Liability limitations (SS 7) | Yes | Correct carve-outs for life/body/health |
| Governing law (SS 9) | Yes | German law, CISG excluded |
| ODR platform reference | Yes | EU ODR link included |
| Salvatory clause | Yes | SS 9(4) |

**Note for B2B:** The AGB correctly apply to both consumers and entrepreneurs (SS 1(3)). The B2B-specific interest rate (9 percentage points) is correctly cited in SS 6(4).

---

## 4. Widerrufsbelehrung (SSSS 312g, 355, 356 BGB)

**Status: PASS**

| Requirement | Present | Notes |
|---|---|---|
| 14-day withdrawal period | Yes | |
| Clear, plain-language text | Yes | |
| Muster-Widerrufsformular (Anlage 1 SS 246a EGBGB) | Yes | Complete form template |
| Early expiry for services (SS 356 Abs. 4) | Yes | Correctly explains parking-specific scenario |
| Contact details placeholders | Yes | |
| Consequences of withdrawal | Yes | Refund timeline and method |

This is one of the strongest templates in the suite. The parking-specific guidance on early expiry of the withdrawal right (when the parking period has started) is particularly well-handled.

---

## 5. Cookie/localStorage Policy (TTDSG SS 25)

**Status: WARN -- gaps found, fixes applied**

| Requirement | Present | Notes |
|---|---|---|
| Authentication token | Yes | `parkhub_token` documented |
| Theme preference | Yes | `parkhub_theme` documented |
| Onboarding hint dismissals | **No** | `parkhub_hint_*` keys -- **FIXED** |
| Feature flags | **No** | `parkhub_features` key -- **FIXED** |
| Use case preference | **No** | `parkhub_usecase` key -- **FIXED** |
| Language preference | **No** | `i18nextLng` key -- **FIXED** |
| Service worker cache | **No** | Cache API usage -- **FIXED** |
| Legal basis (SS 25 Abs. 2 Nr. 2) | Yes | Correctly identified as technically necessary |
| No consent banner required | Yes | Correct for technically necessary storage |

**TTDSG SS 25 analysis of new localStorage keys:**

All new localStorage entries qualify as "technically necessary" under SS 25 Abs. 2 Nr. 2 TTDSG:
- `parkhub_hint_*`: UI state management -- remembers which tooltips the user dismissed. Without this, tooltips would reappear on every page load, degrading usability. No personal data stored (value is just `"1"`).
- `parkhub_features`: Stores the array of enabled feature module names. This is a user preference equivalent to theme/language. No personal data.
- `parkhub_usecase`: Stores the use case selection (`business`/`residential`/`personal`). Required for correct UI rendering.
- `i18nextLng`: Language preference. Standard i18next behavior, technically necessary for rendering content in the user's language.

**None of these require consent.** They contain no personal data and are functionally necessary for the service the user explicitly requested.

**Service worker (sw.js):** Caches static assets only (JS, CSS, fonts, images, icons) and the SPA shell (`/`). API routes (`/api/*`) and health endpoints are explicitly excluded. No personal data is cached by the service worker. This is compliant.

---

## 6. AVV Template (Art. 28 DSGVO)

**Status: PASS**

| Requirement | Present | Notes |
|---|---|---|
| Parties identified | Yes | Placeholders |
| Subject and duration (SS 1) | Yes | |
| Nature and purpose (SS 2) | Yes | Table format |
| Data categories and subjects | Yes | |
| Processor obligations (SS 3) | Yes | Art. 32 reference |
| TOMs (SS 4) | Yes | 7 measure categories |
| Sub-processor clause (SS 5) | Yes | Prior written consent required |
| Data subject rights support (SS 6) | Yes | |
| Breach notification (SS 7) | Yes | 24-hour target |
| Governing law | Yes | German law |

**Recommendation:** Consider adding a clause about auditing rights (Art. 28(3)(h) DSGVO) -- the controller's right to conduct audits or have them conducted. This is a standard AVV element sometimes expected by DPAs.

---

## 7. VVT Template (Art. 30 DSGVO)

**Status: PASS**

| Requirement | Present | Notes |
|---|---|---|
| Controller identity (Art. 30(1)(a)) | Yes | |
| Purpose of processing (Art. 30(1)(b)) | Yes | 5 activities documented |
| Categories of data subjects and data | Yes | Per activity |
| Recipients (Art. 30(1)(d)) | Yes | SMTP conditional |
| International transfers (Art. 30(1)(e)) | Yes | Per activity |
| Retention periods (Art. 30(1)(f)) | Yes | |
| TOMs description (Art. 30(1)(g)) | Yes | Section C |
| Version history | Yes | Section D |
| Bilingual (DE/EN) | Yes | Good for international operators |

**Recommendation:** Add a processing activity entry for "Client-side preference storage" (localStorage) to be thorough, even though localStorage is not server-side processing. Some DPAs interpret Art. 30 broadly.

---

## 8. GDPR.md Operator Guide

**Status: PASS**

Both the Rust and PHP versions are comprehensive and well-structured. They correctly cover:
- Data inventory with retention periods
- All user rights (Art. 15-22) with API endpoints
- DSAR handling procedures
- Pre-production compliance checklist
- Technical and organizational measures
- Cookie/TTDSG analysis
- DSB appointment guidance

**Minor update needed:** The GDPR.md should mention the new localStorage keys from the UX features. **FIXED** -- added to the "What ParkHub Does NOT Collect" section and localStorage coverage.

---

## 9. SECURITY.md and SECURITY-AUDIT.md

**Status: PASS**

These documents are thorough and technically accurate. The OWASP audit from 2026-02-28 correctly identifies the rate-limiting gap and provides operator mitigation guidance. No legal compliance issues found in these documents.

---

## 10. New UX Features -- Compliance Assessment

### 10.1 `onboarding_hints` (localStorage: `parkhub_hint_*`)

**Status: PASS**

- Stores only dismissal state (`"1"`) per hint ID
- Contains zero personal data
- Technically necessary for UX (prevents repeated tooltip display)
- SS 25 Abs. 2 Nr. 2 TTDSG: no consent required
- Data is purely client-side -- never transmitted to the server
- `resetAllHints()` function provides user control

### 10.2 `generative_bg` (CSS-only generative backgrounds)

**Status: PASS** -- No data collection, no compliance impact.

### 10.3 `micro_animations` (framer-motion)

**Status: PASS** -- No data collection, no compliance impact.

### 10.4 `fab_quick_actions` (floating action button)

**Status: PASS** -- No data collection, no compliance impact.

### 10.5 `rich_empty_states` (SVG illustrations)

**Status: PASS** -- No data collection, no compliance impact. SVGs are embedded, not loaded from external CDN.

### 10.6 Feature flags (`parkhub_features` in FeaturesContext)

**Status: PASS with note**

- localStorage key `parkhub_features` stores an array of feature module names (e.g. `["vehicles","credits"]`)
- **Also synced to the server** via `api.updateFeatures()` -- this is a user preference, stored as part of the user profile
- Server-side storage is covered by the existing "User preferences" data category in the GDPR documentation
- The feature list itself contains no sensitive data (just module IDs)
- SS 25 Abs. 2 Nr. 2 TTDSG: technically necessary (controls which UI modules render)

---

## 11. PWA-Specific Privacy Considerations

**Status: PASS**

| Concern | Assessment |
|---|---|
| Service worker caching | Only static assets (JS, CSS, fonts, images) and SPA shell. API routes explicitly excluded. No personal data cached. |
| Cache API persistence | Caches persist until service worker update. Old caches are correctly cleaned up in the `activate` handler. |
| Offline behavior | Falls back to cached SPA shell on network failure. No personal data served from cache. |
| Push subscriptions (PHP only) | Covered in PHP GDPR.md as consent-based (Art. 6 Abs. 1 lit. a). Rust version does not implement push. |
| Install prompt | Standard PWA manifest -- no additional data collection. |

---

## 12. International Compliance Notes

### 12.1 European Union (beyond Germany)

| Regulation | Status | Notes |
|---|---|---|
| GDPR (all EU/EEA) | PASS | On-premise design eliminates cross-border transfer concerns |
| ePrivacy Directive (2002/58/EG) | PASS | No cookies, no tracking. localStorage technically necessary. |
| EU Accessibility Act (EAA) / BFSG | WARN | See Section 13 |

### 12.2 United Kingdom

| Regulation | Status | Notes |
|---|---|---|
| UK GDPR (retained EU law) | PASS | Substantively identical to EU GDPR for on-premise deployment |
| UK DPA 2018 | PASS | No special category data processed |
| PECR (UK ePrivacy) | PASS | Same analysis as TTDSG -- technically necessary storage exempt |

**Note for UK operators:** The Impressum concept does not exist in UK law, but business websites must identify the operator under the Companies Act 2006 (s.82) and the Electronic Commerce (EC Directive) Regulations 2002 (reg. 6). The Impressum template covers these requirements.

### 12.3 United States

| Regulation | Status | Notes |
|---|---|---|
| CCPA/CPRA (California) | PASS | On-premise: operator is sole data controller. No "sale" of data. |
| State privacy laws (Virginia, Colorado, Connecticut, etc.) | PASS | Same analysis -- self-hosted, no third-party sharing |
| ADA (accessibility) | WARN | See Section 13 |

**Note:** US operators do not need the Widerrufsbelehrung or Impressum. The AGB template would need adaptation for US contract law (UCC vs. BGB), but this is appropriately left to the operator.

### 12.4 Switzerland

| Regulation | Status | Notes |
|---|---|---|
| nDSG (new Data Protection Act, 2023) | PASS | Substantively compatible with DSGVO. The Datenschutzerklarung template works with minor adaptations (FDPIC instead of Landesbehorde). |

### 12.5 International Data Transfers

Not applicable for the core system (on-premise). Only relevant when operators configure external SMTP providers. The AVV template correctly addresses this with a placeholder for SCCs (Standard Contractual Clauses) when the SMTP provider is in a third country.

---

## 13. Accessibility (BFSG / BFSG -- Barrierefreiheitsstartkungsgesetz)

**Status: WARN -- documentation gap**

The BFSG (implementing the EU Accessibility Act / EAA) has been in effect since June 28, 2025. It applies to:
- B2C digital services (websites, apps) offered by businesses with >10 employees or >EUR 2M annual turnover
- **Does NOT apply to B2B-only services or micro-enterprises**

ParkHub as open-source software itself is not subject to BFSG, but **operators using it for B2C services may be**. The project already implements several accessibility features (semantic HTML via React, `aria-label` attributes on interactive elements, keyboard navigation), but lacks documentation.

**Recommendation:** Add a brief accessibility note to the GDPR.md or a separate section in the README noting:
- The BFSG/EAA applicability criteria
- What ParkHub already provides (semantic HTML, ARIA labels, keyboard navigation, color contrast via Tailwind)
- What operators should verify (WCAG 2.1 AA compliance testing, screen reader testing)

This is informational only and does not require code changes.

---

## 14. Business vs. Personal Use Compliance Differences

| Requirement | Personal/Private Use | Business (B2B) | Business (B2C) |
|---|---|---|---|
| Impressum (DDG SS 5) | Not required | **Required** | **Required** |
| Datenschutzerklarung | Recommended | **Required** | **Required** |
| AGB | Not required | Recommended | **Strongly recommended** |
| Widerrufsbelehrung | Not applicable | Not applicable | **Required** |
| AVV (for SMTP) | Not required | **Required** if SMTP used | **Required** if SMTP used |
| VVT (Art. 30) | Not required | Required if >=250 employees or risk processing | Required if >=250 employees or risk processing |
| BFSG/EAA accessibility | Not applicable | Not applicable | Required if >10 employees / >EUR 2M turnover |
| SS 147 AO tax retention | Not applicable | **10-year retention** | **10-year retention** |
| Cookie consent banner | Not required (no cookies) | Not required (no cookies) | Not required (no cookies) |
| DSB appointment | Not required | Evaluate per Art. 37 | Evaluate per Art. 37 |

ParkHub's use-case selector (`business`/`residential`/`personal`) is a good UX pattern. The legal templates correctly note which documents are conditional on the operator's situation.

---

## 15. Data Protection Impact Assessment (DPIA)

**Status: WARN -- guidance missing, added to GDPR.md**

A DPIA (Datenschutz-Folgenabschatzung, Art. 35 DSGVO) is required when processing is "likely to result in a high risk to the rights and freedoms of natural persons." For most ParkHub deployments, a DPIA is **not required** because:

- No systematic monitoring of publicly accessible areas
- No large-scale processing of special category data (Art. 9)
- No automated decision-making with legal/significant effects
- No large-scale profiling

However, operators processing data for **large commercial parking operations** (thousands of users, license plate recognition, CCTV integration) should evaluate DPIA necessity. Added a section to GDPR.md.

---

## 16. Right to Erasure vs. Tax Retention Conflict

**Status: PASS -- correctly handled**

The conflict between Art. 17 DSGVO (right to erasure) and SS 147 AO (10-year tax retention) is **correctly resolved** in both repos:
- User PII is anonymized (name, email, username -> `[DELETED]`)
- Booking records are retained with anonymized references
- License plates on bookings are replaced with `[GELOSCHT]`
- The legal basis (Art. 17(3)(b) -- legal obligation) is documented

This is the standard approach recommended by German DPAs.

---

## Summary of Changes Made

### Files updated:

1. **`legal/datenschutz-template.md`** (both repos)
   - Added Art. 7 Abs. 3 consent withdrawal right (Rust version was missing it)
   - Expanded Section 5 to document all localStorage keys

2. **`legal/cookie-policy-template.md`** (both repos)
   - Added sections for onboarding hint dismissals, feature flags, use case preference, and language preference
   - Added service worker / Cache API section
   - Added note about i18next language detection

3. **`docs/GDPR.md`** (Rust repo)
   - Added DPIA guidance section
   - Updated localStorage documentation to include new keys
   - Added BFSG/accessibility note

### No new files created.

All existing templates are legally sound for their purpose as operator-customizable templates. The project correctly disclaims that templates do not constitute legal advice.

---

*This compliance report was generated as part of a structured legal review. It does not constitute legal advice. Operators deploying ParkHub in production should have their specific configuration reviewed by a qualified legal professional.*
