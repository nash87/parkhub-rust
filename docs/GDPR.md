# GDPR / DSGVO Compliance Guide — ParkHub Rust

> **Version:** 3.3.0 | **Last updated:** 2026-04-12

This guide is addressed to operators deploying ParkHub Rust within the European Union (EU)
or the European Economic Area (EEA), where the General Data Protection Regulation
(DSGVO — Datenschutz-Grundverordnung) applies.

**This document is informational and does not constitute legal advice. Consult a
qualified data protection attorney (Datenschutzbeauftragter) for binding guidance.**

---

## Table of Contents

1. [Why On-Premise Simplifies GDPR](#why-on-premise-simplifies-gdpr)
2. [Legal Basis for Data Processing (Art. 6)](#legal-basis-for-data-processing-art-6)
3. [Data Inventory](#data-inventory)
4. [What ParkHub Does NOT Collect](#what-parkhub-does-not-collect)
5. [Information Obligations (Art. 13/14)](#information-obligations-art-1314)
6. [User Rights Implementation](#user-rights-implementation)
7. [Data Retention Configuration](#data-retention-configuration)
8. [Legal Documents](#legal-documents)
9. [Technical and Organizational Measures (Art. 32)](#technical-and-organizational-measures-toms-art-32)
10. [TTDSG §25 — Cookie / localStorage Policy](#cookie-policy-ttdsg-25)
11. [DDG §5 — Impressum Requirement](#ddg-5--impressum-requirement)
12. [Breach Notification (Art. 33/34)](#breach-notification-art-3334)
13. [DSGVO Compliance Checklist](#dsgvo-compliance-checklist)
14. [Responding to DSARs](#responding-to-data-subject-access-requests-dsar)
15. [Data Protection Impact Assessment (DPIA)](#data-protection-impact-assessment-dpia)
16. [Accessibility (BFSG / EU Accessibility Act)](#accessibility-bfsg--eu-accessibility-act)
17. [Data Protection Officer (DSB)](#data-protection-officer-dsb)

---

## Why On-Premise Simplifies GDPR

ParkHub Rust is designed for on-premise, self-hosted deployment. All data remains on your server.

| Aspect | Benefit |
|--------|---------|
| No cloud upload | No Auftragsverarbeitungsvertrag (AVV / Art. 28 DPA) needed for the core system |
| No third-party SaaS | No dependency on external privacy policies |
| Full control | You control storage location, encryption, access, and retention |
| No analytics | No tracking pixels, no CDN, no external JavaScript |
| No third-party processors | Data stays on-premise — Art. 28 does not apply to the core system |

> **Exception**: If you configure SMTP email notifications, your SMTP provider becomes
> a data processor and requires an AVV. A template is in `legal/avv-template.md`.
> If you enable the Stripe module, Stripe acts as an independent data controller for
> payment data under its own privacy policy.

---

## Legal Basis for Data Processing (Art. 6)

ParkHub processes personal data under the following legal bases:

| Processing Activity | Legal Basis | DSGVO Reference | Justification |
|---------------------|-------------|-----------------|---------------|
| User registration and authentication | Contract performance | Art. 6 Abs. 1 lit. b | Account creation is necessary to provide the parking service |
| Booking management | Contract performance | Art. 6 Abs. 1 lit. b | Core service — booking, pricing, invoicing |
| Vehicle data storage | Contract performance | Art. 6 Abs. 1 lit. b | Required for booking association and plate display |
| Absence tracking | Contract performance | Art. 6 Abs. 1 lit. b | Employee parking coordination feature |
| Booking record retention (10 years) | Legal obligation | Art. 6 Abs. 1 lit. c | §147 AO — German tax law retention requirement |
| Audit logging | Legitimate interest | Art. 6 Abs. 1 lit. f | Security monitoring, fraud prevention, accountability |
| Push notifications | Consent | Art. 6 Abs. 1 lit. a | Explicit opt-in via browser permission dialog |
| Payment processing (Stripe) | Contract performance | Art. 6 Abs. 1 lit. b | Required to complete paid bookings |

**Legitimate interest balancing test (Art. 6 lit. f — audit log):**
The operator has a legitimate interest in security monitoring and accountability.
The audit log stores action type, timestamp, user ID (anonymized on erasure), and IP address.
IP addresses are retained for a configurable period (recommended: 90 days). The data subject's
interests do not override the security interest because: (1) no profiling occurs, (2) data
is not shared with third parties, (3) retention is time-limited, (4) anonymization on erasure
is supported.

---

## Data Inventory

### User Accounts (Art. 6 Abs. 1 lit. b DSGVO — contract performance)

| Category | Fields | Retention |
|----------|--------|-----------|
| Identity | `name`, `username`, `email`, `phone`, `picture` | Until deletion / anonymization |
| Role / access | `role`, `is_active`, `department`, `last_login` | Until deletion |
| Preferences | `preferences` JSON (theme, language, timezone, notification settings) | Until deletion |
| Security | `two_factor_secret`, `two_factor_recovery_codes` | Until 2FA disabled or account deleted |

### Booking Records (Art. 6 Abs. 1 lit. b + lit. c DSGVO)

| Fields | Retention |
|--------|-----------|
| `lot_name`, `slot_number`, `vehicle_plate`, `start_time`, `end_time`, `status`, `booking_type`, `notes`, `price`, `currency` | 10 years (§147 AO, anonymized on erasure) |

**German tax law**: §147 AO requires 10-year retention of accounting records for commercial
parking operations. Booking records with pricing data fall under this obligation.

### Vehicle Data (Art. 6 Abs. 1 lit. b DSGVO)

| Fields | Retention |
|--------|-----------|
| `plate`, `make`, `model`, `color`, `photo_url` | Until vehicle deleted / account anonymization |

### Absence Data (Art. 6 Abs. 1 lit. b DSGVO)

| Fields | Retention |
|--------|-----------|
| `absence_type`, `start_date`, `end_date`, `note` | Until deleted / account anonymization |

### Payment Data (Art. 6 Abs. 1 lit. b DSGVO — Stripe module)

| Fields | Retention |
|--------|-----------|
| `stripe_session_id`, `amount`, `currency`, `status`, `booking_id` | 10 years (§147 AO) |

> **Note**: ParkHub stores only transaction references. Full payment card data is processed
> and stored exclusively by Stripe under their PCI-DSS compliance. ParkHub never sees or
> stores card numbers.

### Audit Log (Art. 6 Abs. 1 lit. c + lit. f DSGVO)

| Fields | Retention |
|--------|-----------|
| `user_id`, `username`, `action`, `details`, `ip_address`, `created_at` | Operator-configured (recommended: 90 days to 1 year) |

IP addresses stored in the audit log may constitute personal data under DSGVO. Implement
a retention policy — see the [Audit Log Pruning](#audit-log-pruning) section below.

### Push Subscriptions (Art. 6 Abs. 1 lit. a DSGVO — consent)

| Fields | Retention |
|--------|-----------|
| Browser push endpoint, encryption keys | Until unsubscription |

### Tenant Data (Art. 6 Abs. 1 lit. b DSGVO — Multi-Tenant module)

| Fields | Retention |
|--------|-----------|
| `tenant_name`, `tenant_slug`, `branding` | Until tenant deleted by admin |

---

## What ParkHub Does NOT Collect

| Item | Status |
|------|--------|
| HTTP Cookies | None — `localStorage` is used for session token, theme, language, feature flags, use case, and onboarding hint dismissals (all technically necessary) |
| Analytics / telemetry | None |
| External CDN resources | None — all assets served locally |
| Third-party tracking | None |
| Advertising data | None |
| Fingerprinting | None |
| Location data | None (lot coordinates are operator-configured, not user-tracked) |

No cookie consent banner is required for core functionality (TTDSG §25).

---

## Information Obligations (Art. 13/14)

Operators must provide a privacy notice (Datenschutzerklärung) to data subjects **before**
processing begins. ParkHub provides:

1. **A ready-to-use German privacy notice template**: [`docs/PRIVACY-TEMPLATE.md`](PRIVACY-TEMPLATE.md) / [`legal/datenschutz-template.md`](/legal/datenschutz-template.md)
2. **An admin panel** to publish the privacy notice at `/privacy`
3. **In-app visibility**: Users see a link to the privacy notice during registration and in settings

The privacy notice must include (per Art. 13):
- Controller identity and contact details
- DPO contact (if applicable)
- Purposes and legal basis for each processing activity
- Recipients or categories of recipients
- Retention periods per data category
- Data subject rights (Art. 15–22)
- Right to lodge a complaint with a supervisory authority (Art. 77)
- Whether data provision is a statutory/contractual requirement

All of these are covered in the template. Operators must fill in their organization-specific details.

---

## User Rights Implementation

### Art. 15 — Right of Access (Auskunftsrecht)

Users can view all their data through the application and download a complete JSON export.

**User-facing**: Settings → Export My Data

**API endpoint**: `GET /api/v1/user/export`

The export includes: profile, all bookings, all absences, all vehicles, preferences, payment history.

**Operator action required**: None — fully automated.

---

### Art. 16 — Right to Rectification (Berichtigungsrecht)

Users can update name, email, phone, and department via the Settings page.
Administrators can update any user field via `PUT /api/v1/admin/users/:id`.

**Operator action required**: None.

---

### Art. 17 — Right to Erasure (Recht auf Vergessenwerden)

**API endpoint**: `DELETE /api/v1/users/me/delete`

What this endpoint does:

1. Replaces `name`, `email`, `username`, `phone`, `picture` with `[DELETED]`
2. Replaces `license_plate` on all bookings with `[DELETED]`
3. Deletes all registered vehicles
4. Invalidates all active sessions
5. Booking records are retained (with anonymized references) for accounting purposes

After erasure, booking records remain (with anonymized user reference) for §147 AO
compliance. The user cannot log in again.

---

### Art. 18 — Right to Restriction of Processing

Not automatically implemented. Handle restriction requests manually by deactivating
the user's account (`PUT /api/v1/admin/users/:id` with `is_active: false`) and
documenting the restriction in your internal process log.

---

### Art. 20 — Right to Data Portability (Datenübertragbarkeit)

The export endpoint (`GET /api/v1/user/export`) delivers all personal data in
machine-readable JSON format. This satisfies Art. 20.

For CSV export: `GET /api/v1/admin/reports/export-csv` (admin-initiated).

**Operator action required**: None.

---

### Art. 21 — Right to Object (Widerspruchsrecht)

Users can disable email notifications and push notifications via their preferences.
For processing on the basis of legitimate interest (Art. 6 lit. f — audit log data):
establish an email-based process for objections. Add the contact address to your Impressum.

---

## Data Retention Configuration

### GDPR Retention Days

Set a default data retention period in the admin panel:

Admin → Privacy → Data Retention Days

Or via the API:

```toml
# config.toml
data_retention_days = 730
gdpr_enabled = true
```

### Recommended Retention Periods

| Data Category | Recommended Retention | Legal Basis |
|---------------|----------------------|-------------|
| User accounts | Until deletion request | Art. 6 lit. b |
| Booking records | 10 years | §147 AO |
| Payment records | 10 years | §147 AO |
| Audit logs | 90 days – 1 year | Art. 6 lit. f |
| Push subscriptions | Until unsubscribed | Art. 6 lit. a |
| Vehicle data | Until deleted | Art. 6 lit. b |
| Session tokens | 7 days (auto-expiry) | Art. 6 lit. b |

### Audit Log Pruning

Configure audit log retention in `config.toml`:

```toml
audit_log_retention_days = 90
```

ParkHub Rust handles session expiry internally based on the configured
`session_timeout_minutes` setting. Expired sessions are automatically cleaned up.

---

## Legal Documents

### Impressum (DDG §5)

The Impressum is legally required for all commercial digital services in Germany.

1. Admin panel → Impressum (or `PUT /api/v1/admin/impressum`)
2. Fill in all required fields (provider name, address, email, phone, company register, VAT ID)
3. The Impressum is publicly accessible at `/impressum` and via `GET /api/v1/legal/impressum`

**Required fields**: provider name, legal form, street, postal code, city, country, email,
phone. For GmbH/AG: register court, register number, VAT ID, managing directors.

**Templates**: [`legal/impressum-template.md`](/legal/impressum-template.md) | [`docs/IMPRESSUM-TEMPLATE.md`](IMPRESSUM-TEMPLATE.md)

### Datenschutzerklärung (Privacy Policy)

A DSGVO-compliant privacy policy is required.

**Templates**: [`legal/datenschutz-template.md`](/legal/datenschutz-template.md) | [`docs/PRIVACY-TEMPLATE.md`](PRIVACY-TEMPLATE.md)

Adapt the template to reflect your organization's:
- Name and contact details
- Specific data categories processed
- Any data processor agreements (AVV)
- Data Protection Officer contact (if applicable)

Store the policy text via Admin → Privacy. It is displayed at `/privacy`.

### AGB (Terms of Service)

Required for commercial parking services.

**Template**: `legal/agb-template.md`

### AVV (Auftragsverarbeitungsvertrag — Data Processing Agreement)

Required if you use SMTP providers (SendGrid, Mailgun, Postmark) or any service
that processes personal data on your behalf.

**Template**: `legal/avv-template.md`

---

## Technical and Organizational Measures (TOMs, Art. 32)

| Measure | Implementation in ParkHub Rust |
|---------|------------------------------|
| **Encryption in transit** | HTTPS (operator responsibility — configure TLS at reverse proxy) |
| **Encryption at rest** | Database encryption or OS disk encryption (operator responsibility) |
| **Access control** | RBAC (user/admin/superadmin), session-based auth |
| **Authentication hardening** | 2FA/TOTP with backup codes, configurable password policies |
| **Pseudonymization** | Anonymization endpoint replaces PII with pseudonymous identifiers |
| **Audit logging** | All write operations logged with user, action, IP, timestamp |
| **Brute-force protection** | Rate limiting on login (10/min), registration (10/min), password reset (5/15min) |
| **Data minimization** | Only required fields collected; preferences user-controlled |
| **Password security** | Argon2id with configurable parameters |
| **File upload validation** | MIME type and size validation |
| **Injection prevention** | No SQL (embedded redb database); typed Rust API with no string interpolation |
| **XSS prevention** | React JSX auto-escaping, CSP headers via Tower middleware |
| **CSRF protection** | Bearer token auth (SPA) — no cookie-based CSRF applicable |
| **Memory safety** | Written in Rust — no buffer overflows, no use-after-free |
| **Security headers** | CSP, HSTS, X-Frame-Options, X-Content-Type-Options via Tower middleware |
| **Session management** | Token rotation, list/revoke sessions, auto-expiry |
| **Privacy by design (Art. 25)** | Self-hosted architecture — no third-party data processors by default |

Organizational measures (privacy by default, staff training, incident response,
DPA registration) are the operator's responsibility.

---

## Cookie Policy (TTDSG §25)

ParkHub Rust does not set any cookies. The following `localStorage` entries are used — all are
technically necessary and do not require consent under TTDSG §25 Abs. 2 Nr. 2:

| Key | Content | Purpose |
|-----|---------|---------|
| `parkhub_token` | Bearer session token | Authentication |
| `parkhub_theme` | `light` / `dark` / `system` | Display preference |
| `parkhub_features` | Array of enabled module IDs | Feature configuration |
| `parkhub_usecase` | `business` / `residential` / `personal` | UI preset selection |
| `parkhub_hint_*` | `1` (dismissed) | Onboarding tooltip state |
| `i18nextLng` | Language code (e.g. `de`) | Language preference |

The PWA service worker caches static assets only (JS, CSS, fonts, images). API responses
and user data are **never** cached by the service worker.

No analytics, advertising, or tracking technologies are used.

**TTDSG §25 analysis**: All localStorage entries qualify as "technically necessary" under
§25 Abs. 2 Nr. 2 TTDSG. They contain no personal data (except the authentication token,
which is deleted on logout) and are required for the service the user explicitly requested.
**No consent banner is required.**

---

## DDG §5 — Impressum Requirement

The Digitale-Dienste-Gesetz (DDG, formerly TMG) §5 requires all commercial digital service
providers in Germany to publish an Impressum with:

- Provider name and legal form
- Full postal address
- Direct contact (email, phone recommended)
- Handelsregister entry (if applicable)
- USt-IdNr. (if VAT-liable, per §27a UStG)
- Responsible person for content (§18 Abs. 2 MStV)

ParkHub implements this via:
- **Admin panel**: Admin → Impressum
- **API**: `PUT /api/v1/admin/impressum` / `GET /api/v1/legal/impressum`
- **Public URL**: `/impressum` — accessible without authentication
- **Template**: [`docs/IMPRESSUM-TEMPLATE.md`](IMPRESSUM-TEMPLATE.md)

---

## Breach Notification (Art. 33/34)

### Art. 33 — Notification to Supervisory Authority

In case of a personal data breach, the operator (as data controller) must notify the
competent supervisory authority **within 72 hours** of becoming aware of the breach,
unless the breach is unlikely to result in a risk to the rights and freedoms of
natural persons.

### Art. 34 — Communication to Data Subjects

If the breach is likely to result in a **high risk** to the rights and freedoms of
natural persons, the operator must also notify affected data subjects without undue delay.

### Breach Notification Template

```
DATENSCHUTZVERLETZUNG — MELDUNG NACH ART. 33 DSGVO

An: [Zuständige Aufsichtsbehörde]
Von: [Verantwortlicher — Name, Adresse]
Datum der Meldung: [Datum]
Datum der Kenntnisnahme: [Datum]

1. ART DER VERLETZUNG
   [ ] Vertraulichkeit (unbefugter Zugriff)
   [ ] Integrität (unbefugte Änderung)
   [ ] Verfügbarkeit (Datenverlust)

2. BETROFFENE DATENKATEGORIEN
   [ ] Nutzerdaten (Name, E-Mail, Benutzername)
   [ ] Buchungsdaten (Parkplatz, Zeitraum, Kennzeichen)
   [ ] Zahlungsdaten (Transaktionsreferenzen)
   [ ] Protokolldaten (IP-Adressen)

3. UNGEFÄHRE ANZAHL BETROFFENER PERSONEN: [Anzahl]

4. WAHRSCHEINLICHE FOLGEN: [Beschreibung]

5. ERGRIFFENE MAßNAHMEN:
   [ ] Zugangsdaten zurückgesetzt
   [ ] Betroffene Nutzer benachrichtigt
   [ ] Sicherheitslücke geschlossen
   [ ] [Weitere Maßnahmen]

6. DATENSCHUTZBEAUFTRAGTER: [Name, Kontakt]
```

### ParkHub Features Supporting Breach Response

| Feature | Endpoint | Purpose |
|---------|----------|---------|
| Audit log | `GET /api/v1/admin/audit-log` | Determine scope of unauthorized access |
| Session management | `GET /api/v1/admin/sessions` | Identify active sessions |
| Token revocation | Admin → Users → Revoke tokens | Force re-authentication |
| User export | `GET /api/v1/user/export` | Identify affected data |
| Rate limit history | `GET /api/v1/admin/rate-limits/history` | Detect brute-force patterns |

---

## DSGVO Compliance Checklist

Before going live:

**Legal setup**
- [ ] Impressum fully filled in (Admin → Impressum). Verify `/impressum` is publicly accessible
- [ ] Datenschutzerklärung written and published (Admin → Privacy → Policy Text)
- [ ] AGB created and published (if commercial service)
- [ ] AVV signed with SMTP provider (e.g. Mailgun, SendGrid, Postmark) — `legal/avv-template.md`
- [ ] AVV signed with hosting provider if they can physically access your server
- [ ] Verzeichnis der Verarbeitungstätigkeiten (VVT) updated (Art. 30 DSGVO)
- [ ] DPA/DSB appointment evaluated (Art. 37 DSGVO)
- [ ] Widerrufsbelehrung published (if B2C commercial service)

**Technical controls**
- [ ] HTTPS enabled (TLS 1.2+ at reverse proxy)
- [ ] `APP_DEBUG=false` and `APP_ENV=production` in `.env`
- [ ] Disk encryption at the OS level for the data volume
- [ ] GDPR enabled: `gdpr_enabled=true` in admin settings
- [ ] Data retention policy for audit logs implemented
- [ ] Backup encryption configured
- [ ] Access logging at reverse proxy level

**Testing**
- [ ] Export endpoint tested: `GET /api/v1/user/export` → verify JSON completeness
- [ ] Anonymization endpoint tested: verify PII removed, booking records retained
- [ ] Hard deletion tested: verify CASCADE removes all user data
- [ ] Password reset flow tested end-to-end
- [ ] 2FA enrollment and recovery tested

---

## Responding to Data Subject Access Requests (DSAR)

When a user submits a DSAR:

**For Art. 15/20 (access / portability) requests:**
1. Verify the requester's identity
2. Call `GET /api/v1/user/export` on their behalf (as admin)
3. Deliver within 30 calendar days

**For Art. 17 (erasure) requests:**
1. Verify identity
2. Call `POST /api/v1/users/me/anonymize` on their behalf
3. Inform the user of the §147 AO retention limitation and its legal basis
4. Document in your internal DSAR log

**For Art. 18 (restriction) requests:**
1. Verify identity
2. Deactivate account via `PUT /api/v1/admin/users/:id` with `is_active: false`
3. Document the restriction and review date

---

## Data Protection Impact Assessment (DPIA)

A DPIA (Art. 35 DSGVO) is required when data processing is "likely to result in a high
risk to the rights and freedoms of natural persons."

**For most ParkHub deployments, a DPIA is NOT required** because:

- No systematic monitoring of publicly accessible areas (unless combined with CCTV)
- No large-scale processing of special category data (Art. 9)
- No automated decision-making with legal or similarly significant effects
- No large-scale profiling

**Consider conducting a DPIA if your deployment involves:**

- Large-scale commercial parking operations (thousands of daily users)
- License plate recognition (ALPR/ANPR) integration
- CCTV or camera-based monitoring of parking areas
- Combining parking data with employee monitoring or access control systems
- Processing data of vulnerable persons (e.g., hospital parking for patients)

If a DPIA is required, use your supervisory authority's DPIA template. The data inventory
in this document and the VVT template (`legal/vvt-template.md`) provide the necessary input.

---

## Accessibility (BFSG / EU Accessibility Act)

The Barrierefreiheitsstärkungsgesetz (BFSG), implementing the EU Accessibility Act (EAA),
has been in effect since June 28, 2025. It applies to B2C digital services offered by
businesses with more than 10 employees or more than EUR 2 million annual turnover.

**ParkHub as open-source software is not itself subject to BFSG**, but operators using it
for consumer-facing services may be. ParkHub provides:

- Semantic HTML structure (via React components)
- ARIA labels on interactive elements (buttons, navigation, form fields)
- Keyboard navigation support
- High-contrast color schemes (light and dark theme)
- Responsive design for all screen sizes
- Lighthouse CI accessibility gate (>= 95)

**Operators should independently verify** WCAG 2.1 Level AA compliance for their specific
deployment, particularly screen reader compatibility and custom content accessibility.

---

## Data Protection Officer (DSB)

Under Art. 37 DSGVO, appointing a Data Protection Officer is mandatory for:
- Public bodies and authorities
- Organizations processing special categories of data (Art. 9) at scale
- Organizations systematically monitoring individuals at large scale

Under German law (§38 BDSG), appointment is also mandatory when at least 20 persons are
regularly engaged in the automated processing of personal data.

For most organizations using ParkHub for internal parking management, appointment is
not mandatory. Consult your legal advisor to determine your obligation.

---

*This guide covers DSGVO compliance for ParkHub Rust v3.3.0. For international compliance
(UK GDPR, CCPA, nDSG, LGPD), see [COMPLIANCE.md](COMPLIANCE.md). For security architecture,
see [SECURITY.md](SECURITY.md).*
