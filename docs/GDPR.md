# GDPR / DSGVO Operator Compliance Guide — ParkHub Rust

This guide is addressed to operators deploying ParkHub in the European Union (EU) or
European Economic Area (EEA), where the General Data Protection Regulation (DSGVO —
Datenschutz-Grundverordnung) applies.

**This document is informational and does not constitute legal advice. Consult a
qualified data protection attorney (Datenschutzbeauftragter) for binding guidance.**

---

## Why On-Premise Simplifies GDPR

ParkHub is designed for on-premise, self-hosted deployment. All data stays on your server.

| Aspect | Benefit |
|--------|---------|
| No cloud upload | No Auftragsverarbeitungsvertrag (AVV) needed for the core system |
| No third-party SaaS | No dependency on external privacy policies (AWS, Google, etc.) |
| Full control | You control storage location, encryption, access, and retention |
| Zero CDN | All assets are embedded in the server binary — no external requests at runtime |

> **Exception**: If you configure SMTP email notifications, your SMTP provider becomes
> a data processor and requires an AVV. A template is provided in `legal/avv-template.md`.

---

## Data Inventory

### User Accounts (Art. 6 Abs. 1 lit. b DSGVO — contract performance)

| Data Field | Purpose | Retention |
|-----------|---------|-----------|
| Name | Display in UI, booking records | Until account deletion |
| Email address | Login, notifications | Until account deletion |
| Username | Login identifier | Until account deletion |
| Password hash (Argon2id) | Authentication. No plaintext stored | Until account deletion |
| Role (user/admin/superadmin) | Access control | Until account deletion |
| Account creation date | Audit trail | Until account deletion |
| Last login timestamp | Security audit | Until account deletion |
| User preferences | Personalization (language, theme, default vehicle) | Until account deletion |

### Booking Records (Art. 6 Abs. 1 lit. b + lit. c DSGVO)

| Data Field | Purpose | Retention |
|-----------|---------|-----------|
| Booking ID, slot ID, lot ID | Unique booking identification | 10 years (§147 AO) |
| Licence plate | Proof of parking authorization | 10 years (§147 AO, anonymized on erasure) |
| Start and end time | Contract fulfillment | 10 years (§147 AO) |
| Price, tax, currency | Accounting record | 10 years (§147 AO) |
| Status (confirmed/cancelled/completed) | Contract state | 10 years (§147 AO) |
| Creation timestamp | Booking log | 10 years (§147 AO) |

**German tax law**: §147 AO (Abgabenordnung) requires 10-year retention of accounting
records. Booking records with pricing data fall under this obligation for commercial
operations. The erasure endpoint anonymizes PII but retains the booking record structure.

### Vehicle Data (Art. 6 Abs. 1 lit. b DSGVO)

| Data Field | Purpose | Retention |
|-----------|---------|-----------|
| Licence plate | Booking linkage | Until vehicle deleted / account erasure |
| Make, model, color | Vehicle identification (optional) | Until vehicle deleted |

### Technical Log Data (Art. 6 Abs. 1 lit. f DSGVO — legitimate interest)

| Data | Purpose | Recommended Retention |
|------|---------|----------------------|
| IP addresses in HTTP request logs | Security, abuse prevention | 30 days (configure via log rotation) |
| Audit log entries (login, booking, deletion) | Security audit trail | 90 days to 1 year |

---

## What ParkHub Does NOT Collect

| Item | Status |
|------|--------|
| Cookies | None — only `localStorage` is used for the Bearer token (technically necessary) |
| Analytics / tracking pixels | None |
| External CDN requests | None — all assets embedded in the binary |
| External font requests | None |
| Third-party scripts | None |
| Advertising IDs | None |

ParkHub therefore requires no cookie consent banner for core functionality (TTDSG §25).

---

## User Rights Implementation

### Art. 15 — Right of Access (Auskunftsrecht)

Users can download all their personal data as a JSON file.

**User-facing path**: Profile → Privacy → Export My Data

**API endpoint**: `GET /api/v1/users/me/export`

The export includes: user profile, all bookings, all vehicles. Password hash is
intentionally excluded from exports.

**Operator action required**: None. Implemented out of the box.

---

### Art. 16 — Right to Rectification (Berichtigungsrecht)

Users can update their profile (name, email) via the Settings page.
Administrators can update any user field via `PATCH /api/v1/admin/users/:id`.

**Operator action required**: None. Implemented out of the box.

---

### Art. 17 — Right to Erasure (Recht auf Vergessenwerden)

**API endpoint**: `DELETE /api/v1/users/me/delete`

What the endpoint does:

1. Replaces `name`, `email`, `username`, `phone`, `picture` with `[DELETED]`
2. Deletes all registered vehicles
3. Retains booking records but replaces `license_plate` with `[DELETED]`
   (§147 AO compliance — 10-year retention of accounting records)
4. Invalidates all active sessions

Why booking records are not fully deleted: German tax law (§147 AO) requires
10-year retention of booking records for commercial parking operations. The anonymization
procedure removes all personal identifiers while satisfying the retention obligation.

**Operator action required**: None. However, consider documenting this retention
rationale in your Datenschutzerklärung (Privacy Policy).

---

### Art. 18 — Right to Restriction of Processing

Not automatically implemented. Handle restriction requests manually by deactivating
the user's account (`PATCH /api/v1/admin/users/:id/status` with `is_active: false`)
and documenting the restriction in your internal processes.

---

### Art. 20 — Right to Data Portability (Datenübertragbarkeit)

The Art. 15 export (`GET /api/v1/users/me/export`) delivers data in machine-readable
JSON format. This satisfies Art. 20.

**Operator action required**: None.

---

### Art. 21 — Right to Object (Widerspruchsrecht)

For processing on the basis of legitimate interest (Art. 6 lit. f — log data):
Establish an email-based process for objections. Add the contact address to your Impressum.

---

## Data Retention Configuration

### Licence Plate Display

Restrict how licence plates are displayed to other users:

```toml
# config.toml
# 0 = show full plate
# 1 = blur
# 2 = redact (display as ***)
# 3 = hide entirely
license_plate_display = 2
```

### Self-Registration

Default is `false` — users are created only by administrators:

```toml
allow_self_registration = false
```

### Session Timeout

```toml
session_timeout_minutes = 60   # 0 = never expire
```

### Audit Logging

```toml
audit_logging_enabled = true
```

Logs: logins, booking creation/cancellation, account deletion events.

### Log Retention

ParkHub outputs logs to stdout via `tracing`. Configure log retention at the
infrastructure level (Docker log rotation, journald, or a log aggregator):

```yaml
# docker-compose.yml logging configuration
services:
  parkhub:
    logging:
      driver: json-file
      options:
        max-size: "100m"
        max-file: "7"
```

---

## Impressum (DDG §5)

The Impressum (Provider Identification) is legally required for all commercial digital
services in Germany. ParkHub makes it easy:

1. Log in as an administrator
2. Navigate to **Admin → Impressum** (or call `PUT /api/v1/admin/impressum`)
3. Fill in all required fields
4. The Impressum is automatically available at `/impressum` (no login required)

**Required fields** (DDG §5):
- Provider name and legal form
- Full postal address (street, postal code, city, country)
- Email address
- Phone number
- For GmbH/AG: company register court and number, VAT ID, managing directors

**Template**: `legal/impressum-template.md`

---

## Legal Document Templates

| File | Content |
|------|---------|
| `legal/impressum-template.md` | DDG §5 provider identification |
| `legal/datenschutz-template.md` | DSGVO-compliant privacy policy |
| `legal/agb-template.md` | General Terms and Conditions |
| `legal/avv-template.md` | Data Processing Agreement (for SMTP providers) |

Adapt all templates to your organization's specific situation. For legally binding
wording, consult an attorney specializing in IT law (IT-Recht).

---

## Technical and Organizational Measures (TOMs, Art. 32 DSGVO)

| Measure | Implementation in ParkHub Rust |
|---------|-------------------------------|
| Encryption in transit | TLS 1.3 (auto-generated or custom cert) |
| Encryption at rest | AES-256-GCM (optional, PBKDF2-SHA256 key derivation) |
| Pseudonymization | Configurable licence plate display; anonymization on erasure |
| Access control | RBAC (user/admin/superadmin), session timeout, rate limiting |
| Integrity | Argon2id passwords; no SQL injection surface (redb embedded, no SQL) |
| Availability | Health endpoints, automatic daily backups, Docker restart policy |
| Audit trail | Structured audit log for all security-relevant events |
| Memory safety | Written in Rust — no buffer overflows, no use-after-free |
| Supply chain | All dependencies compiled into a static musl binary |

---

## Pre-Production Checklist (DSGVO Compliance)

Before going live with ParkHub in a production environment:

**Legal setup**
- [ ] Impressum fully filled in (Admin → Impressum). Verify `/impressum` is publicly reachable
- [ ] Datenschutzerklärung (Privacy Policy) drafted, adapted, and published
- [ ] AGB created and published (if paid parking / commercial service)
- [ ] AVV signed with SMTP provider (if email notifications are enabled)
- [ ] Verzeichnis der Verarbeitungstätigkeiten (VVT) updated per Art. 30 DSGVO

**Technical security**
- [ ] Default admin password changed from `admin` to a strong unique password
- [ ] AES-256-GCM encryption enabled (`encryption_enabled = true`, strong `PARKHUB_DB_PASSPHRASE`)
- [ ] TLS active (own cert, reverse proxy, or auto-generated self-signed)
- [ ] `allow_self_registration = false` (unless open registration is intentional)
- [ ] Licence plate display setting reviewed (`license_plate_display`)
- [ ] Session timeout configured (`session_timeout_minutes`)
- [ ] Audit logging enabled (`audit_logging_enabled = true`)
- [ ] Log retention policy implemented (rotate logs, discard after 30–90 days)

**Operations**
- [ ] Backup strategy implemented and tested (automatic daily + off-site copy)
- [ ] Data export tested: `GET /api/v1/users/me/export` → verify JSON completeness
- [ ] Data erasure tested: `DELETE /api/v1/users/me/delete` → verify PII removed, bookings retained
- [ ] Health endpoints return 200: `/health/live`, `/health/ready`

---

## Responding to Data Subject Access Requests (DSAR)

When a user submits a DSAR (Art. 15 or Art. 20 request):

1. **Verify identity** — confirm the request comes from the account holder (e.g. via the registered email)
2. **Export data** — use `GET /api/v1/users/me/export` or export as admin on their behalf
3. **Provide within 30 days** — DSGVO requires a response within 1 calendar month
4. **Format** — the JSON export is machine-readable and satisfies both Art. 15 and Art. 20

For erasure requests (Art. 17):
- Use `DELETE /api/v1/users/me/delete` (anonymizes, retains booking records per §147 AO)
- Document the erasure in your internal DSAR handling log
- Inform the user of the retention limitation and its legal basis (§147 AO)

---

## Cookie Policy (TTDSG §25)

ParkHub does not set any cookies. The Bearer token is stored in `localStorage` — this is
technically necessary for session management and does not require consent under TTDSG §25.

No tracking, analytics, or advertising technologies are used.

---

## Data Protection Officer (DSB)

Under Art. 37 DSGVO, appointing a Data Protection Officer (Datenschutzbeauftragter) is
mandatory for:
- Public authorities
- Organizations processing special categories of data (Art. 9) at scale
- Organizations systematically monitoring individuals at large scale

For most small-to-medium organizations using ParkHub for internal parking management,
appointment is not mandatory. Consult your legal advisor.
