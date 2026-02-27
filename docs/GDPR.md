# GDPR / DSGVO Operator Compliance Guide

This guide is for operators deploying ParkHub for users in the European Union,
in particular for German-regulated environments.

---

## Warum On-Premise DSGVO-Konformität vereinfacht

ParkHub wird On-Premise betrieben — alle Daten bleiben auf Ihrem eigenen Server.

Das bedeutet:

- **Kein Auftragsverarbeitungsvertrag (AVV) mit Drittanbietern** für die Kerndaten, weil keine
  Daten an Cloud-Dienste oder Dritte übermittelt werden.
- **Volle Kontrolle über Speicherort, Verschlüsselung und Zugriff** auf alle personenbezogenen Daten.
- **Keine Abhängigkeit** von externen Datenschutzrichtlinien (AWS, Google Cloud, etc.).

> Ausnahme: Wenn Sie SMTP für E-Mail-Benachrichtigungen konfigurieren, wird Ihr SMTP-Anbieter
> zum Auftragsverarbeiter und benötigt einen AVV. Vorlage: `legal/avv-template.md`.

---

## Welche Daten werden gespeichert

### Nutzerkonto (Art. 6 Abs. 1 lit. b DSGVO — Vertragserfüllung)

| Datenfeld | Zweck |
|---|---|
| Name | Identifizierung, Anzeige im UI |
| E-Mail-Adresse | Kontozugang, Benachrichtigungen |
| Benutzername | Login-Kennung |
| Passwort-Hash (Argon2id) | Authentifizierung. Kein Klartext-Passwort gespeichert |
| Rolle (user/admin/superadmin) | Zugriffskontrolle |
| Erstellungsdatum | Systemprotokoll |
| Letzter Login | Sicherheitsprotokoll |
| Nutzerpräferenzen | Personalisierung (Sprache, Theme, Standard-Fahrzeug) |

### Buchungsdaten (Art. 6 Abs. 1 lit. b DSGVO — Vertragserfüllung)

| Datenfeld | Zweck |
|---|---|
| Buchungs-ID, Slot-ID, Lot-ID | Eindeutige Identifizierung der Buchung |
| Kennzeichen des Fahrzeugs | Nachweis der Parkberechtigung |
| Start- und Endzeit | Vertragsdurchführung |
| Preis, Steuer, Währung | Abrechnungsnachweis |
| Status (confirmed/cancelled/completed) | Vertragszustand |
| Erstellungszeitpunkt | Buchungsprotokoll |

**Aufbewahrungspflicht**: Buchungsbelege unterliegen §147 AO (10 Jahre Aufbewahrung für
steuerrelevante Unterlagen). Die Löschfunktion anonymisiert das Kennzeichen, löscht aber
die Buchungsrecords nicht vollständig.

### Fahrzeugdaten (Art. 6 Abs. 1 lit. b DSGVO)

| Datenfeld | Zweck |
|---|---|
| Kennzeichen | Buchungsverknüpfung |
| Marke, Modell, Farbe | Fahrzeugidentifizierung (optional) |

### Technische Protokolldaten (Art. 6 Abs. 1 lit. f DSGVO — berechtigtes Interesse)

- IP-Adressen aus HTTP-Requests (in der `RUST_LOG`-Ausgabe)
- Speicherdauer: Konfigurierbar über Log-Rotation, empfohlen 30 Tage

### Keine weiteren Daten

ParkHub verwendet:
- **Keine Cookies** (nur `localStorage` für den Auth-Token — technisch notwendig)
- **Kein Google Analytics** oder andere Tracking-Tools
- **Keine CDN-Ressourcen** — alle Assets sind im Binary eingebettet
- **Keine externen Fonts** oder API-Aufrufe

---

## Nutzerrechte und ihre Umsetzung

### Art. 15 — Auskunftsrecht

Nutzer können unter **Profil → Datenschutz → Daten exportieren** (oder via API) alle
ihre gespeicherten Daten als JSON-Datei herunterladen.

**API-Endpunkt**: `GET /api/v1/users/me/export`

Der Export enthält: Profil, alle Buchungen, alle Fahrzeuge. Das Passwort-Hash wird
aus Sicherheitsgründen bewusst ausgeschlossen.

### Art. 16 — Berichtigungsrecht

Nutzer können Profilfelder (Name, E-Mail) in den Einstellungen selbst ändern.
Admins können alle Nutzerfelder bearbeiten.

### Art. 17 — Löschungsrecht (Recht auf Vergessenwerden)

**API-Endpunkt**: `DELETE /api/v1/users/me/delete`

Was die Löschfunktion tut:
1. Name, E-Mail, Benutzername, Telefon, Profilbild → ersetzt durch `[DELETED]`
2. Alle registrierten Fahrzeuge werden vollständig gelöscht
3. Buchungsrecords bleiben erhalten, aber das Kennzeichen wird zu `[DELETED]`
4. Alle aktiven Sessions werden ungültig

**Warum werden Buchungen nicht vollständig gelöscht?**
Deutsche Steuerrecht (§147 AO) schreibt eine 10-jährige Aufbewahrungspflicht für
Buchungsbelege vor. Die Anonymisierung entfernt alle personenbezogenen Informationen
aus den Buchungsrecords bei gleichzeitiger Einhaltung der Aufbewahrungspflicht.

### Art. 18 — Einschränkung der Verarbeitung

Nicht automatisiert implementiert. Handhaben Sie Anfragen zur Einschränkung manuell
über die Datenbank-Administration oder durch Deaktivierung des Benutzerkontos.

### Art. 20 — Datenübertragbarkeit

Der Art.-15-Export (`GET /api/v1/users/me/export`) liefert die Daten im maschinenlesbaren
JSON-Format — erfüllt Art. 20.

### Art. 21 — Widerspruchsrecht

Für Verarbeitungen auf Basis berechtigten Interesses (Art. 6 lit. f — Protokolldaten):
Richten Sie ein Verfahren für Widersprüche per E-Mail ein (Kontaktadresse im Impressum).

---

## Konfiguration zum Datenschutz

### Kennzeichen-Anzeige einschränken

In `config.toml` (oder über die Benutzeroberfläche):

```toml
# 0 = anzeigen (Standard)
# 1 = unscharf anzeigen
# 2 = zensieren (zeigt ***)
# 3 = ausblenden
license_plate_display = 2
```

### Selbstregistrierung deaktivieren

```toml
allow_self_registration = false
```

Standardmäßig deaktiviert. Neue Nutzer werden ausschließlich vom Administrator angelegt.

### Session-Timeout

```toml
session_timeout_minutes = 60  # 0 = nie
```

### Audit-Logging

```toml
audit_logging_enabled = true
```

Protokolliert sicherheitsrelevante Ereignisse (Login, Buchungserstellung, Kontolöschung).

---

## Impressum konfigurieren (DDG §5)

Das Impressum ist gesetzlich vorgeschrieben und muss frei zugänglich sein.

1. Melden Sie sich als Administrator an
2. Navigieren Sie zu **Admin → Impressum** (oder `PUT /api/v1/admin/impressum`)
3. Füllen Sie alle Pflichtfelder aus (Anbieter, Anschrift, E-Mail)
4. Das Impressum ist automatisch unter `/impressum` öffentlich erreichbar

**Vorlage**: `legal/impressum-template.md`

---

## Datenschutzerklärung und AGB

ParkHub liefert fertige Vorlagen im Verzeichnis `legal/`:

| Datei | Inhalt |
|---|---|
| `legal/impressum-template.md` | Pflichtangaben nach DDG §5 |
| `legal/datenschutz-template.md` | DSGVO-konforme Datenschutzerklärung |
| `legal/agb-template.md` | Allgemeine Geschäftsbedingungen (BGB §§305-310) |
| `legal/avv-template.md` | Auftragsverarbeitungsvertrag (falls SMTP genutzt wird) |

Passen Sie die Vorlagen an Ihre konkrete Situation an. Für rechtssichere Formulierungen
empfehlen wir die Beratung durch einen auf IT-Recht spezialisierten Rechtsanwalt.

---

## Checkliste vor dem Produktivbetrieb

- [ ] Admin-Passwort von `admin` auf ein starkes Passwort geändert
- [ ] Impressum vollständig ausgefüllt (`/impressum` erreichbar und vollständig)
- [ ] Datenschutzerklärung erstellt und verlinkt
- [ ] AGB erstellt (falls Buchungen gegen Entgelt)
- [ ] Verschlüsselung aktiviert (`encryption_enabled = true`, starkes `PARKHUB_DB_PASSPHRASE`)
- [ ] TLS aktiv (eigenes Zertifikat oder über Reverse Proxy)
- [ ] `allow_self_registration` auf `false` (sofern kein offenes System gewünscht)
- [ ] Kennzeichen-Anzeige-Einstellung geprüft (`license_plate_display`)
- [ ] Session-Timeout gesetzt (`session_timeout_minutes`)
- [ ] Backup-Strategie implementiert (automatische Backups + Off-Site-Kopie)
- [ ] Audit-Logging aktiviert
- [ ] Verzeichnis der Verarbeitungstätigkeiten (VVT) aktualisiert (Art. 30 DSGVO)
- [ ] Falls SMTP verwendet: AVV mit E-Mail-Anbieter abgeschlossen

---

## Technische Schutzmaßnahmen (Art. 32 DSGVO)

| Maßnahme | Umsetzung in ParkHub |
|---|---|
| Pseudonymisierung | Kennzeichen-Anzeige konfigurierbar, Anonymisierung bei Kontolöschung |
| Verschlüsselung | TLS 1.3 für Transport, AES-256-GCM optional für Datenbank at rest |
| Verfügbarkeit | Health-Endpunkte, automatische Backups, Docker Restart-Policy |
| Integrität | Argon2id für Passwörter, keine SQL-Injection-Angriffsfläche (redb) |
| Zugangskontrolle | RBAC (user/admin/superadmin), Session-Timeout, Rate Limiting |
| Protokollierung | Audit-Logging, strukturiertes Tracing mit `tracing-subscriber` |
