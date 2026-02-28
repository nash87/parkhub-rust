# Verzeichnis der Verarbeitungstätigkeiten (VVT)
# Record of Processing Activities (Art. 30 DSGVO / GDPR)

> **Anleitung für Betreiber:** Füllen Sie alle mit `[...]` markierten Felder aus.
> Das VVT muss schriftlich geführt und auf Anfrage der Aufsichtsbehörde vorgelegt werden.
> Aktualisieren Sie das Verzeichnis bei jeder wesentlichen Änderung der Verarbeitungstätigkeiten.
>
> **Operator instructions:** Fill in all fields marked `[...]`.
> The RoPA must be kept in writing and made available to supervisory authorities on request.

---

## A. Angaben zum Verantwortlichen / Controller Identity

| Feld | Wert |
|------|------|
| **Name / Firma** | [Name des Verantwortlichen / Company name] |
| **Anschrift** | [Straße, PLZ, Ort] |
| **E-Mail** | [datenschutz@ihre-domain.de] |
| **Telefon** | [+49 ...] |
| **Datenschutzbeauftragter (DSB)** | [Name, E-Mail — oder: "Nicht bestellt (Schwellenwert nach § 38 BDSG nicht erreicht)"] |

---

## B. Verarbeitungstätigkeiten / Processing Activities

---

### 1. Nutzerkonto-Verwaltung / User Account Management

| Merkmal | Beschreibung | English annotation |
|---------|-------------|-------------------|
| **Zweck** | Erstellung und Verwaltung von Benutzerkonten, Authentifizierung | Account creation, authentication |
| **Rechtsgrundlage** | Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung) | Contract performance |
| **Datenkategorien** | Benutzername, E-Mail-Adresse, Passwort-Hash (Argon2id), Rolle, Erstellungsdatum | Username, email, password hash, role, created_at |
| **Betroffene** | Mitarbeiter, Nutzer der Parkanlage | Employees, parking users |
| **Empfänger** | Keine Übermittlung an Dritte | No third-party disclosure |
| **Drittlandtransfer** | Keiner (On-Premise) | None (on-premise) |
| **Speicherdauer** | Bis zur Kontolöschung (Art. 17 DSGVO) | Until account deletion |
| **Technische Maßnahmen** | TLS 1.3, Argon2id-Passwort-Hashing, RBAC, Audit-Log | TLS 1.3, Argon2id, RBAC, audit log |

---

### 2. Buchungsverwaltung / Booking Management

| Merkmal | Beschreibung | English annotation |
|---------|-------------|-------------------|
| **Zweck** | Durchführung von Parkplatzbuchungen, Abrechnung, Doppelbuchungsschutz | Parking reservation, billing, race condition protection |
| **Rechtsgrundlage** | Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung); §147 AO (Aufbewahrung) | Contract performance; German tax retention obligation |
| **Datenkategorien** | Buchungs-ID, Benutzer-ID, Parkhaus, Stellplatz, Kennzeichen, Zeitraum, Status | Booking ID, user ID, lot, slot, plate number, period, status |
| **Betroffene** | Nutzer der Parkanlage | Parking users |
| **Empfänger** | Keine (interner Betrieb) | None (internal operation) |
| **Drittlandtransfer** | Keiner | None |
| **Speicherdauer** | 10 Jahre ab Buchungsende (§ 147 AO); PII wird bei Kontolöschung anonymisiert | 10 years (tax law); PII anonymized on account deletion |
| **Technische Maßnahmen** | Write-Lock (Transaktionsschutz), AES-256-GCM-Datenverschlüsselung (optional), Audit-Log | Write-lock, optional AES-256-GCM encryption, audit log |

---

### 3. E-Mail-Benachrichtigungen / Email Notifications

| Merkmal | Beschreibung | English annotation |
|---------|-------------|-------------------|
| **Zweck** | Buchungsbestätigung, Willkommens-E-Mail, Passwort-Zurücksetzen | Booking confirmation, welcome email, password reset |
| **Rechtsgrundlage** | Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung / vorvertragliche Maßnahmen) | Contract performance / pre-contractual measures |
| **Datenkategorien** | E-Mail-Adresse, Name, Buchungsdetails, Passwort-Reset-Token (zeitlich begrenzt) | Email, name, booking details, time-limited reset token |
| **Betroffene** | Registrierte Nutzer | Registered users |
| **Empfänger** | SMTP-Anbieter: [Ihr Anbieter, z. B. Postfix/eigener Server oder externer Dienst + AVV] | SMTP provider (own server or external with DPA) |
| **Drittlandtransfer** | [Ja/Nein — abhängig vom SMTP-Anbieter; ggf. SCCs erforderlich] | Depends on SMTP provider |
| **Speicherdauer** | Reset-Token: max. 1 Stunde; E-Mail-Protokolle: 30 Tage | Reset token: 1h max; mail logs: 30 days |
| **Technische Maßnahmen** | TLS für SMTP (STARTTLS/SMTPS), Token-Einmalnutzung | TLS for SMTP, single-use token |

---

### 4. Audit-Protokollierung / Audit Logging

| Merkmal | Beschreibung | English annotation |
|---------|-------------|-------------------|
| **Zweck** | Sicherheitsüberwachung, Nachvollziehbarkeit von Änderungen, Compliance | Security monitoring, change accountability, compliance |
| **Rechtsgrundlage** | Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse: IT-Sicherheit und Compliance) | Legitimate interest: IT security and compliance |
| **Datenkategorien** | Benutzer-ID, Aktion (Login, Buchung, Löschung usw.), Zeitstempel, IP-Adresse | User ID, action type, timestamp, IP address |
| **Betroffene** | Alle Nutzer (auch Admins) | All users including admins |
| **Empfänger** | Keine (intern); ggf. Aufsichtsbehörde auf Anfrage | None; supervisory authority on request |
| **Drittlandtransfer** | Keiner | None |
| **Speicherdauer** | 90 Tage (rollierend), konfigurierbar | 90 days rolling, configurable |
| **Technische Maßnahmen** | Schreibgeschützte Logs, AES-256-GCM-Verschlüsselung der Datenbank | Read-only logs, encrypted database |

---

### 5. Datensicherung / Database Backups

| Merkmal | Beschreibung | English annotation |
|---------|-------------|-------------------|
| **Zweck** | Datensicherung und Wiederherstellung bei Systemausfall | Data recovery from system failure |
| **Rechtsgrundlage** | Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse: Betriebskontinuität); Art. 32 DSGVO | Legitimate interest: business continuity; Art. 32 GDPR security obligation |
| **Datenkategorien** | Vollständige Datenbank (enthält alle o. g. Kategorien) | Full database snapshot |
| **Betroffene** | Alle Nutzer | All users |
| **Empfänger** | [Backup-Speicherort: lokal / externer Server / Cloud — ggf. AVV erforderlich] | Backup destination; DPA required for cloud |
| **Drittlandtransfer** | [Abhängig vom Backup-Ziel] | Depends on backup destination |
| **Speicherdauer** | 7 tägliche Backups (konfigurierbar über `backup_retention_count`) | 7 daily backups (configurable) |
| **Technische Maßnahmen** | AES-256-GCM-Verschlüsselung, verschlüsselter Transfer (rsync/SCP über TLS) | AES-256-GCM encryption, encrypted transfer |

---

## C. Allgemeine technisch-organisatorische Maßnahmen (TOM) / General TOMs

| Maßnahme | Umsetzung |
|----------|-----------|
| Verschlüsselung (Transport) | TLS 1.3 für alle HTTP-Verbindungen |
| Verschlüsselung (Ruhezustand) | Optional: AES-256-GCM für die redb-Datenbank (PBKDF2-SHA256 Key Derivation) |
| Passwort-Sicherheit | Argon2id mit kryptografisch zufälligen Salts (OsRng) |
| Zugriffskontrolle | RBAC: Rollen `user`, `admin`, `superadmin`; Endpoint-Level-Prüfung |
| Authentifizierung | Session-Token (256-Bit CSPRNG), 24-Stunden-Ablauf (konfigurierbar) |
| Rate Limiting | Login: 5/min, Registrierung: 3/min, Passwort-Reset: 3/15min pro IP |
| Sicherheits-Header | CSP, HSTS (`max-age=31536000; includeSubDomains; preload`), X-Frame-Options, Referrer-Policy |
| Minimierung | Keine Weitergabe an Dritte; kein externes CDN; kein Tracking |
| Protokollierung | Strukturiertes Audit-Log für sicherheitsrelevante Ereignisse |
| Datenlöschung | Art.-17-Endpunkt: PII-Anonymisierung; Buchungen bleiben (§ 147 AO) |

---

## D. Versionierung / Version History

| Version | Datum | Änderung |
|---------|-------|----------|
| 1.0 | [Datum] | Erstversion / Initial version |
| 1.1 | 2026-02-28 | E-Mail-Verarbeitung ergänzt; Audit-Log-Retention konkretisiert |

---

*Vorlage für ParkHub-Betreiber — kein Rechtsrat. Diese Vorlage deckt die Standard-Verarbeitungstätigkeiten
von ParkHub v1.1.0 ab. Individuelle Konfigurationen (externe SMTP-Dienste, Cloud-Backups, zusätzliche
Integrationen) müssen gesondert erfasst werden. Art. 30 DSGVO verpflichtet Verantwortliche mit
≥ 250 Mitarbeitern oder risikoreichen Verarbeitungen zur Führung dieses Verzeichnisses.*
