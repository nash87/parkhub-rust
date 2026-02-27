# Datenschutzerklärung-Vorlage — ParkHub

> **Anleitung für Betreiber:** Passen Sie diese Vorlage an Ihre Installation an und
> hinterlegen Sie die URL Ihrer Datenschutzerklärung in der App-Konfiguration.
>
> **Rechtliche Grundlage:** Art. 13/14 DSGVO, § 25 TTDSG (Datenschutzgesetz für Telemedien)

---

## Datenschutzerklärung

**Stand:** [Datum]

**Verantwortlicher:**
[Name/Firma, Adresse, E-Mail — gleiche Angaben wie im Impressum]

---

## 1. Erhobene Daten und Verarbeitungszwecke

### 1.1 Registrierung und Nutzerkonto

**Daten:** Name, E-Mail-Adresse, Benutzername, verschlüsseltes Passwort
**Zweck:** Kontoerstellung, Authentifizierung, Zugriffskontrolle
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung)
**Speicherdauer:** Bis zur Löschung des Nutzerkontos

### 1.2 Buchungsdaten

**Daten:** Parkhaus, Stellplatz, Zeitraum, Kennzeichen, Buchungs-ID
**Zweck:** Durchführung der Parkplatzbuchung, Abrechnung
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung)
**Speicherdauer:** 10 Jahre (§ 147 AO — gesetzliche Aufbewahrungspflicht)

> **Hinweis:** Buchungseinträge werden nach Löschung eines Nutzerkontos anonymisiert
> (Kennzeichen → [GELÖSCHT]), aber nicht vollständig gelöscht, da steuerrechtliche
> Aufbewahrungspflichten bestehen.

### 1.3 Fahrzeugdaten

**Daten:** Kennzeichen, Fahrzeugmarke, Modell, Farbe
**Zweck:** Vereinfachte Buchung, Kennzeichenerkennung
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO
**Speicherdauer:** Bis zur Löschung des Fahrzeugs oder des Nutzerkontos

### 1.4 Protokolldaten (Server-Logs)

**Daten:** IP-Adresse, Zeitstempel, aufgerufene URL, HTTP-Statuscode
**Zweck:** Betriebssicherheit, Fehleranalyse
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse)
**Speicherdauer:** 30 Tage (rollierend)

---

## 2. Keine Weitergabe an Dritte

**ParkHub wird On-Premise betrieben.** Alle Daten verbleiben auf den Servern des
Betreibers. Es findet keine Übermittlung an externe Dienstleister, Cloud-Dienste oder
Dritte statt, sofern der Betreiber keine externen E-Mail-Dienste (SMTP) konfiguriert hat.

Falls E-Mail-Benachrichtigungen aktiviert sind:
- **SMTP-Anbieter:** [Ihr SMTP-Anbieter und Datenschutzhinweis]
- **Übermittelte Daten:** Name, E-Mail, Buchungsdetails

---

## 3. Ihre Rechte (Art. 15–22 DSGVO)

| Recht | Beschreibung |
|-------|-------------|
| **Auskunft** (Art. 15) | Sie können jederzeit alle über Sie gespeicherten Daten exportieren (Profil → Datenschutz → Daten exportieren) |
| **Berichtigung** (Art. 16) | Korrekturen von Profildaten unter Einstellungen |
| **Löschung** (Art. 17) | Kontoauflösung unter Profil → Konto löschen (anonymisiert Buchungen, löscht PII) |
| **Einschränkung** (Art. 18) | Auf Anfrage an [E-Mail] |
| **Datenübertragbarkeit** (Art. 20) | Export als JSON-Datei verfügbar |
| **Widerspruch** (Art. 21) | Für auf berechtigtem Interesse beruhende Verarbeitungen |
| **Beschwerde** | Zuständige Aufsichtsbehörde: [Ihr Bundesland-Datenschutzbeauftragter] |

---

## 4. Technische Sicherheit

- **Transport:** TLS 1.3 (HTTPS)
- **Passwort-Hashing:** Argon2id (bei Rust-Version) / bcrypt (bei PHP-Version)
- **Datenverschlüsselung (Rust):** Optional: AES-256-GCM at rest für die redb-Datenbank
- **Authentifizierung:** JWT-Token / Laravel Sanctum
- **Keine Cookies für Tracking:** Nur technisch notwendige Session-Tokens

---

## 5. Cookies und lokaler Speicher

**Technisch notwendige Token:**
ParkHub verwendet localStorage (keine Cookies) für den Authentifizierungstoken.
Dieser Token ist technisch notwendig für den Betrieb und enthält keine Tracking-Informationen.

**Keine Analyse-Cookies, keine Werbe-Cookies, kein Google Analytics.**

---

## 6. Kontakt Datenschutz

Bei Fragen zur Datenverarbeitung wenden Sie sich an:

**Datenschutzbeauftragter (DSB) / Kontakt:**
[Name]
[E-Mail für Datenschutzanfragen]
[Telefon]

---

*Vorlage für ParkHub-Betreiber — kein Rechtsrat. Diese Vorlage deckt gängige Szenarien ab,
ersetzt aber keine individuelle rechtliche Beratung.*
