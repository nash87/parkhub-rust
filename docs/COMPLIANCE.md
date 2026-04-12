# Legal Compliance Matrix — ParkHub Rust

> **Version:** 3.3.0 | **Last updated:** 2026-04-12

This document maps ParkHub Rust features to legal requirements across German, EU, and
international data protection regulations. It is intended for operators evaluating ParkHub
for deployment in regulated environments.

**This document is informational and does not constitute legal advice.**

---

## Table of Contents

1. [German Law](#german-law)
2. [EU Law](#eu-law)
3. [International Law](#international-law)
4. [Data Processing Categories](#data-processing-categories)
5. [Data Retention Policies](#data-retention-policies)
6. [Sub-Processor List](#sub-processor-list)
7. [Compliance Checklist](#compliance-checklist)

---

## German Law

### DSGVO (Datenschutz-Grundverordnung)

The German implementation of the EU GDPR. ParkHub's self-hosted architecture means the
operator is the sole data controller with no mandatory data processor agreements for
core functionality.

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| Art. 5 — Data processing principles | Data minimization (only required fields), purpose limitation (parking management), storage limitation (configurable retention) | Core |
| Art. 6 — Legal basis | Documented per processing activity in [GDPR.md](GDPR.md) | Core |
| Art. 7 — Conditions for consent | Push notifications require explicit browser consent; all other processing based on contract/legal obligation | Web Push module |
| Art. 12 — Transparent communication | Privacy notice template with plain-language explanations | `legal/datenschutz-template.md` |
| Art. 13/14 — Information obligations | Ready-to-use Datenschutzerklärung template | [PRIVACY-TEMPLATE.md](PRIVACY-TEMPLATE.md) |
| Art. 15 — Right of access | `GET /api/v1/user/export` — full data export | GDPR module |
| Art. 16 — Right to rectification | Profile editing via Settings page and API | Core |
| Art. 17 — Right to erasure | `DELETE /api/v1/users/me/delete` (anonymization with booking retention) | GDPR module |
| Art. 18 — Right to restriction | Admin can deactivate accounts | Admin module |
| Art. 20 — Data portability | JSON export via `/api/v1/user/export`, CSV via admin reports | GDPR + Data Export modules |
| Art. 21 — Right to object | Notification preferences toggle, contact form for objections | Notifications module |
| Art. 25 — Privacy by design | Self-hosted = no third-party processors; only required fields collected | Architecture |
| Art. 28 — Processor agreements | No DPA needed for core (on-premise); AVV template for SMTP | `legal/avv-template.md` |
| Art. 30 — Records of processing | VVT template with all processing activities | `legal/vvt-template.md` |
| Art. 32 — Security of processing | See [SECURITY.md](SECURITY.md) — encryption, access control, audit logging | Core |
| Art. 33/34 — Breach notification | Breach notification template in [GDPR.md](GDPR.md); audit log for forensics | Core |
| Art. 35 — DPIA | Guidance in GDPR.md; not required for typical deployments | Documentation |
| Art. 37 — DPO appointment | Guidance provided; operator responsibility | Documentation |

### TTDSG (Telekommunikation-Telemedien-Datenschutz-Gesetz)

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| §25 Abs. 1 — Consent for non-essential storage | Not applicable — ParkHub uses no cookies and no non-essential localStorage | Core |
| §25 Abs. 2 Nr. 2 — Technically necessary exemption | All localStorage entries (token, theme, features, language, hints) qualify as technically necessary | Core |
| §25 — Cookie consent banner | **Not required** — no tracking cookies, no analytics, no advertising | Architecture |

### DDG (Digitale-Dienste-Gesetz)

Replaced the TMG (Telemediengesetz) as of 2024.

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| §5 — Impressum (provider identification) | Admin panel for Impressum, public display at `/impressum`, API endpoint | Admin Settings |
| §6 — Special Impressum requirements | Template covers all GmbH/AG fields, VAT ID, register court | `legal/impressum-template.md` |

### TMG (Telemediengesetz) — Legacy

Superseded by DDG. ParkHub references DDG §5 (not TMG §5) in all templates.

### BDSG (Bundesdatenschutzgesetz)

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| §38 — DPO appointment threshold | Guidance in GDPR.md (20+ employees processing personal data) | Documentation |
| §26 — Employee data processing | Applicable if ParkHub is used for employee parking; legal basis: Art. 6 lit. b (employment contract) | Core |

### GoBD (Grundsätze ordnungsmäßiger Buchführung und Dokumentation)

Relevant for operators using ParkHub for commercial parking with revenue tracking.

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| Traceability | Audit log records all booking/payment operations with timestamps | Audit Log |
| Immutability | Audit log has no delete endpoint; deletion requires direct DB access | Audit Log |
| Retention | Booking records retained for 10 years (§147 AO) with anonymized PII | Core |
| Completeness | All state-changing operations logged | Audit Log |
| Export | CSV export for booking and revenue data | Admin Reports + Data Export |
| PDF invoices | Per-booking invoice generation with sequential numbering | Invoices module |

---

## EU Law

### GDPR (General Data Protection Regulation)

Identical to DSGVO coverage above. ParkHub complies with all GDPR requirements through
its self-hosted architecture and built-in privacy features.

### ePrivacy Directive (2002/58/EC)

| Requirement | How ParkHub Complies | Module / Feature |
|-------------|---------------------|------------------|
| Art. 5(3) — Cookie consent | No cookies used; localStorage is technically necessary | Core |
| Art. 13 — Unsolicited communications | Push notifications require explicit opt-in; email only for transactional messages | Web Push module |

### NIS2 Directive (Network and Information Systems)

NIS2 applies to essential and important entities in the EU. Most parking operators fall
outside NIS2 scope unless they are part of critical infrastructure (e.g., airport parking,
hospital parking). However, ParkHub supports NIS2-aligned security practices:

| NIS2 Requirement | How ParkHub Supports | Module / Feature |
|------------------|---------------------|------------------|
| Risk management | Security headers, rate limiting, input validation | Tower security middleware |
| Incident handling | Audit log, breach notification template | Audit Log |
| Business continuity | Single binary + Docker, embedded redb database with automatic backups | Deployment |
| Supply chain security | All Rust crate dependencies MIT/Apache-2.0; no proprietary dependencies | [LICENSE-THIRD-PARTY.md](/LICENSE-THIRD-PARTY.md) |
| Encryption | Argon2id passwords, TLS 1.3 in transit, AES-256-GCM encryption at rest | Core |
| Vulnerability disclosure | Security advisory process via GitHub | [SECURITY.md](/SECURITY.md) |
| Multi-factor authentication | 2FA/TOTP with QR enrollment and backup codes | Auth module |

### EU Accessibility Act (EAA) / BFSG

See [GDPR.md](GDPR.md#accessibility-bfsg--eu-accessibility-act) for BFSG applicability
and ParkHub's accessibility features.

---

## International Law

### UK GDPR (United Kingdom)

The UK retained the GDPR post-Brexit as the UK GDPR, alongside the Data Protection Act 2018.

| Requirement | How ParkHub Complies | Notes |
|-------------|---------------------|-------|
| UK GDPR — all articles | Substantively identical to EU GDPR | Same features apply |
| UK DPA 2018 | No special category data processed | No additional requirements |
| PECR (Privacy and Electronic Communications Regulations) | Same analysis as TTDSG — technically necessary storage exempt | No consent banner needed |
| Companies Act 2006 s.82 | Impressum template covers required business identification | Operator adapts template |

**Note for UK operators:** The Impressum concept does not exist in UK law, but business
websites must identify the operator under the Companies Act 2006 and the Electronic Commerce
(EC Directive) Regulations 2002 (reg. 6). The Impressum template covers these requirements.

### CCPA / CPRA (California, USA)

The California Consumer Privacy Act (CCPA) and California Privacy Rights Act (CPRA).

| Requirement | How ParkHub Complies | Notes |
|-------------|---------------------|-------|
| Right to know | `GET /api/v1/user/export` provides full data disclosure | Automated |
| Right to delete | `DELETE /api/v1/users/me/delete` | Automated |
| Right to opt-out of sale | **Not applicable** — ParkHub does not sell personal data | Architecture |
| Right to non-discrimination | No tiered service based on privacy choices | Architecture |
| Data inventory disclosure | Data categories documented in this file and GDPR.md | Documentation |
| Privacy notice | Datenschutzerklärung template adaptable for CCPA | Template |

**Note**: For CCPA compliance, US operators should adapt the privacy notice template to
include CCPA-specific language (categories of PI collected, business purpose, "Do Not Sell"
disclosure). ParkHub's self-hosted model means operators are sole data controllers with no
"sale" of personal information.

### nDSG (Switzerland — New Data Protection Act)

The Swiss nDSG (in force since September 1, 2023) aligns closely with the GDPR.

| Requirement | How ParkHub Complies | Notes |
|-------------|---------------------|-------|
| Art. 6 — Privacy by design | Self-hosted architecture, data minimization | Architecture |
| Art. 7 — Privacy by default | Only essential fields required; modules opt-in | Module system |
| Art. 19 — Information duty | Datenschutzerklärung template adaptable for Switzerland | Replace DPA references with FDÖB/EDÖB |
| Art. 25 — Right of access | `GET /api/v1/user/export` | Automated |
| Art. 32 — Right to data portability | JSON export | Automated |
| Art. 24 — Breach notification | 72-hour notification template | Documentation |

**Note for Swiss operators:** Replace references to German Landesbehörde with the EDÖB
(Eidgenössischer Datenschutz- und Öffentlichkeitsbeauftragter).

### LGPD (Brazil — Lei Geral de Proteção de Dados)

The Brazilian LGPD closely mirrors the GDPR.

| Requirement | How ParkHub Complies | Notes |
|-------------|---------------------|-------|
| Art. 7 — Legal basis | Mapped from GDPR Art. 6 — contract performance and legitimate interest | Core |
| Art. 9 — Sensitive data | ParkHub does not process sensitive data (biometric, health, etc.) | Architecture |
| Art. 15 — Data termination | Account deletion and anonymization endpoints | GDPR module |
| Art. 18 — Data subject rights | Export, rectification, deletion — same as GDPR implementation | Core |
| Art. 46 — International transfer | Not applicable for on-premise deployments | Architecture |
| Art. 50 — Security practices | Technical measures documented in SECURITY.md | Documentation |

**Note for Brazilian operators:** Adapt the privacy notice template to reference LGPD
articles instead of DSGVO. Replace the DPA complaint reference with the ANPD (Autoridade
Nacional de Proteção de Dados).

---

## Data Processing Categories

| Category | Data Fields | Legal Basis | Retention | Encryption |
|----------|-------------|-------------|-----------|------------|
| **User data** | name, email, username, phone, picture, department, role | Art. 6 lit. b | Until deletion | Argon2id (password), TLS (transit) |
| **Booking data** | lot, slot, vehicle plate, times, status, type, price | Art. 6 lit. b + c | 10 years (§147 AO) | TLS (transit) |
| **Payment data** | Stripe session ID, amount, currency, status | Art. 6 lit. b | 10 years (§147 AO) | Stripe PCI-DSS |
| **Vehicle data** | plate, make, model, color, photo | Art. 6 lit. b | Until deleted | TLS (transit) |
| **Absence data** | type, dates, note | Art. 6 lit. b | Until deleted | TLS (transit) |
| **Audit logs** | user ID, action, details, IP, timestamp | Art. 6 lit. f | 90 days – 1 year | TLS (transit) |
| **Push subscriptions** | browser endpoint, keys | Art. 6 lit. a | Until unsubscribed | TLS (transit) |
| **Tenant data** | name, slug, branding | Art. 6 lit. b | Until deleted | TLS (transit) |

---

## Data Retention Policies

All retention periods are configurable by the operator. The following are recommended defaults:

| Category | Default Retention | Configurable | Legal Minimum |
|----------|-------------------|--------------|---------------|
| User accounts | Until deletion request | Yes | None |
| Booking records | 10 years | Yes (admin panel) | 10 years (§147 AO for commercial) |
| Payment records | 10 years | Yes | 10 years (§147 AO) |
| Vehicle data | Until deleted by user | Yes | None |
| Absence data | Until deleted by user | Yes | None |
| Audit logs | 90 days | Yes (cron config) | None (recommended: 90 days min) |
| Push subscriptions | Until unsubscribed | Automatic | None |
| Session tokens | 7 days | Yes (`SESSION_LIFETIME`) | None |
| Login history | 90 days | Yes | None |

---

## Sub-Processor List

### Self-Hosted Deployment (Default)

**No sub-processors.** All data is processed exclusively on the operator's infrastructure.
No Auftragsverarbeitungsvertrag (AVV / DPA) is required for the core system.

### Optional Sub-Processors (if enabled by operator)

| Service | Purpose | Module | Data Shared | DPA Required |
|---------|---------|--------|-------------|--------------|
| **SMTP provider** (e.g., Mailgun, SendGrid, Postmark) | Email notifications | Core (optional) | Name, email, booking details | Yes — use `legal/avv-template.md` |
| **Stripe** | Payment processing | Stripe module | Payment amount, currency, email | Yes — Stripe provides their own DPA |

### Cloud/PaaS Deployment

If deploying on a PaaS platform (Render, Railway, Fly.io), the platform provider becomes
a sub-processor:

| Platform | DPA Available | Notes |
|----------|---------------|-------|
| Render | Yes (render.com/privacy) | US-based; EU SCCs available |
| Railway | Yes (railway.app/legal) | US-based; EU SCCs available |
| Fly.io | Yes (fly.io/legal) | US-based; EU region available |
| IONOS / Hetzner | Yes | German hosting, no international transfer |
| All-Inkl | Yes | German hosting, no international transfer |

---

## Compliance Checklist

### Pre-Launch (All Deployments)

- [ ] Privacy notice published (adapted from template)
- [ ] Impressum published (German operators)
- [ ] HTTPS configured with TLS 1.2+
- [ ] `RUST_LOG=warn` in production
- [ ] AES-256-GCM database encryption enabled
- [ ] Admin password changed from default
- [ ] Rate limiting verified on login/register endpoints
- [ ] Data export endpoint tested
- [ ] Account deletion/anonymization tested
- [ ] Backup strategy documented

### German Law Specific

- [ ] Impressum complete per DDG §5 (name, address, email, phone, register, VAT ID)
- [ ] Datenschutzerklärung published with all Art. 13 elements
- [ ] AGB published (if B2C commercial service)
- [ ] Widerrufsbelehrung published (if B2C with consumers)
- [ ] VVT (Art. 30 records) created and maintained
- [ ] AVV signed with SMTP provider (if email enabled)
- [ ] DSB appointment evaluated (§38 BDSG)
- [ ] §147 AO retention configured for booking/payment records

### EU / International

- [ ] Data subject rights accessible (export, deletion, rectification)
- [ ] Breach notification process documented
- [ ] Cookie/localStorage policy documented (even if no banner needed)
- [ ] Third-party sub-processors listed in privacy notice
- [ ] International transfer safeguards (SCCs) for non-EU sub-processors
- [ ] NIS2 self-assessment completed (if in scope)
- [ ] BFSG/EAA accessibility assessment (if B2C with >10 employees)

### CCPA Specific (California)

- [ ] Privacy notice includes CCPA-required disclosures
- [ ] "Do Not Sell" disclosure included (even if not applicable)
- [ ] Consumer request process documented (45-day response window)

---

*This compliance matrix covers ParkHub Rust v3.3.0. It does not constitute legal advice.
Operators should have their specific deployment reviewed by qualified legal professionals
in their jurisdiction.*
