# Datenschutzerklarung — Vorlage fur ParkHub-Betreiber

> **Anleitung:** Ersetzen Sie alle Platzhalter `[...]` mit Ihren echten Daten.
> Hinterlegen Sie die fertige Datenschutzerklarung unter **Admin -> Datenschutz -> Datenschutztext**.
> Sie wird automatisch unter `/privacy` fur alle Besucher angezeigt.
>
> **Rechtliche Grundlage:** Art. 13/14 DSGVO, SS 25 TTDSG
>
> **Hinweis:** Diese Vorlage ist kein Rechtsrat. Lassen Sie die fertige Datenschutzerklarung
> von einem qualifizierten Datenschutzbeauftragten oder Rechtsanwalt prufen.

---

## Datenschutzerklarung

**Stand:** [Datum, z. B. 01.01.2026]

### Verantwortlicher (Art. 4 Nr. 7 DSGVO)

**[Firmenname / Vor- und Nachname]**
[Rechtsform, z. B. GmbH, UG (haftungsbeschrankt), Einzelunternehmer]

[Strasse und Hausnummer]
[PLZ Ort]
[Land]

**E-Mail:** [datenschutz@ihre-domain.de]
**Telefon:** [+49 XXX XXXXXXX]

### Datenschutzbeauftragter (falls bestellt)

**[Name des DSB]**
**E-Mail:** [dsb@ihre-domain.de]
**Telefon:** [+49 XXX XXXXXXX]

> Hinweis: Ein Datenschutzbeauftragter ist nach SS 38 BDSG Pflicht, wenn mindestens
> 20 Personen standig mit der automatisierten Verarbeitung personenbezogener Daten
> beschaftigt sind. Siehe auch Art. 37 DSGVO.

---

## 1. Erhobene Daten und Verarbeitungszwecke

### 1.1 Registrierung und Nutzerkonto

**Daten:** Name, E-Mail-Adresse, Benutzername, verschlusseltes Passwort, Telefonnummer (optional), Profilbild (optional), Abteilung (optional)
**Zweck:** Kontoerstellung, Authentifizierung, Zugriffskontrolle
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO (Vertragserfullung)
**Speicherdauer:** Bis zur Loschung des Nutzerkontos

### 1.2 Buchungsdaten

**Daten:** Parkhaus, Stellplatz, Zeitraum, Kennzeichen, Buchungs-ID, Buchungstyp, Preis, Wahrung, Notizen
**Zweck:** Durchfuhrung der Parkplatzbuchung, Abrechnung
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO (Vertragserfullung), Art. 6 Abs. 1 lit. c (gesetzliche Aufbewahrungspflicht)
**Speicherdauer:** 10 Jahre (SS 147 AO -- gesetzliche Aufbewahrungspflicht fur Geschaftsunterlagen)

> **Hinweis:** Buchungseintr\u00e4ge werden nach Loschung eines Nutzerkontos anonymisiert
> (Kennzeichen -> [GELOSCHT], Name -> [GELOSCHT]), aber nicht vollstandig geloscht,
> da steuerrechtliche Aufbewahrungspflichten bestehen (Art. 17 Abs. 3 lit. b DSGVO).

### 1.3 Fahrzeugdaten

**Daten:** Kennzeichen, Fahrzeugmarke, Modell, Farbe, Fahrzeugfoto (optional)
**Zweck:** Vereinfachte Buchung, Kennzeichenanzeige, Fahrzeugidentifikation
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO
**Speicherdauer:** Bis zur Loschung des Fahrzeugs oder des Nutzerkontos

### 1.4 Abwesenheitsdaten

**Daten:** Abwesenheitstyp (Homeoffice, Urlaub, Krankheit), Start-/Enddatum, Notiz
**Zweck:** Teamubersicht, Parkplatzplanung
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO
**Speicherdauer:** Bis zur Loschung oder Kontoanonymisierung

### 1.5 Zahlungsdaten (falls Stripe-Modul aktiviert)

**Daten:** Transaktions-ID, Betrag, Wahrung, Zahlungsstatus
**Zweck:** Zahlungsabwicklung fur kostenpflichtige Buchungen
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. b DSGVO
**Speicherdauer:** 10 Jahre (SS 147 AO)

> **Hinweis:** Kreditkartendaten werden ausschliesslich von Stripe (Stripe, Inc.) verarbeitet
> und gespeichert. ParkHub speichert keine Kartennummern. Stripe ist PCI-DSS zertifiziert.
> Datenschutzhinweise von Stripe: https://stripe.com/de/privacy

### 1.6 Protokolldaten (Audit Log)

**Daten:** Benutzer-ID, Aktion, Details (JSON), IP-Adresse, Zeitstempel
**Zweck:** Betriebssicherheit, Missbrauchserkennung, Nachvollziehbarkeit
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse an IT-Sicherheit)
**Speicherdauer:** [90 Tage / 1 Jahr -- bitte anpassen]

### 1.7 Push-Benachrichtigungen (falls aktiviert)

**Daten:** Browser-Push-Endpunkt, Verschlusselungsschlussel
**Zweck:** Echtzeit-Benachrichtigungen uber Buchungsanderungen
**Rechtsgrundlage:** Art. 6 Abs. 1 lit. a DSGVO (Einwilligung)
**Speicherdauer:** Bis zum Widerruf der Einwilligung (Abmeldung)

> Die Einwilligung kann jederzeit uber die Browser-Einstellungen oder in der App
> unter Einstellungen -> Benachrichtigungen widerrufen werden.

---

## 2. Keine Weitergabe an Dritte

**ParkHub wird On-Premise betrieben.** Alle Daten verbleiben auf den Servern des
Betreibers. Es findet keine Ubermittlung an externe Dienstleister, Cloud-Dienste oder
Dritte statt, sofern nicht nachfolgend aufgefuhrt.

### Ausnahmen (falls zutreffend -- nicht Zutreffendes streichen):

**E-Mail-Benachrichtigungen:**
- SMTP-Anbieter: [Name des Anbieters, z. B. Mailgun, SendGrid, Postmark]
- Ubermittelte Daten: Name, E-Mail-Adresse, Buchungsdetails
- AVV: Auftragsverarbeitungsvertrag liegt vor
- Datenschutzhinweise: [URL des Anbieters]

**Zahlungsabwicklung (Stripe):**
- Anbieter: Stripe, Inc., 354 Oyster Point Blvd, South San Francisco, CA 94080, USA
- Ubermittelte Daten: E-Mail-Adresse, Zahlungsbetrag
- Rechtsgrundlage fur Drittlandubermittlung: EU-Standardvertragsklauseln (SCCs)
- Datenschutzhinweise: https://stripe.com/de/privacy

---

## 3. Ihre Rechte (Art. 15-22 DSGVO)

| Recht | Beschreibung | Umsetzung in ParkHub |
|-------|-------------|---------------------|
| **Auskunft** (Art. 15) | Vollstandige Auskunft uber gespeicherte Daten | Einstellungen -> Daten exportieren (`GET /api/v1/user/export`) |
| **Berichtigung** (Art. 16) | Korrektur unrichtiger Daten | Einstellungen -> Profil bearbeiten |
| **Loschung** (Art. 17) | Loschung aller personenbezogenen Daten | Einstellungen -> Konto loschen (anonymisiert Buchungen, loscht PII) |
| **Einschrankung** (Art. 18) | Einschrankung der Verarbeitung | Auf Anfrage an [E-Mail] |
| **Datenubertragbarkeit** (Art. 20) | Export in maschinenlesbarem Format | JSON-Export unter Einstellungen verfugbar |
| **Widerspruch** (Art. 21) | Widerspruch gegen Verarbeitung auf Basis berechtigter Interessen | Kontakt: [E-Mail] |
| **Widerruf der Einwilligung** (Art. 7 Abs. 3) | Widerruf erteilter Einwilligungen mit Wirkung fur die Zukunft | Push: Browser-Einstellungen; E-Mail: Einstellungen |

### Beschwerderecht (Art. 77 DSGVO)

Sie haben das Recht, sich bei einer Aufsichtsbehorde zu beschweren.

**Zustandige Aufsichtsbehorde:**
[Name der Landesbehorde, z. B.:]
- Bayern: Bayerisches Landesamt fur Datenschutzaufsicht (BayLDA), Promenade 18, 91522 Ansbach
- NRW: Landesbeauftragte fur Datenschutz und Informationsfreiheit NRW (LDI NRW)
- BW: Landesbeauftragter fur den Datenschutz und die Informationsfreiheit (LfDI BW)
- Berlin: Berliner Beauftragte fur Datenschutz und Informationsfreiheit
- Niedersachsen: Landesbeauftragte fur den Datenschutz Niedersachsen (LfD)

Vollstandige Liste: https://www.bfdi.bund.de/DE/Infothek/Anschriften_Links/anschriften_links-node.html

---

## 4. Technische Sicherheitsmassnahmen

- **Transport:** TLS 1.2+ (HTTPS) -- verschlusselte Datenubertragung
- **Passwort-Hashing:** bcrypt mit 12 Runden (konfigurierbar)
- **Zwei-Faktor-Authentifizierung:** TOTP mit QR-Code und Backup-Codes
- **Zugriffskontrolle:** 4-stufiges Rollensystem (Nutzer, Premium, Admin, Superadmin)
- **Audit-Log:** Alle sicherheitsrelevanten Aktionen werden protokolliert
- **Rate Limiting:** Schutz gegen Brute-Force-Angriffe bei Login und Registrierung
- **Dateivalidierung:** GD-basierte Inhaltsvalidierung fur Bild-Uploads
- **SQL-Injection-Schutz:** Eloquent ORM mit parametrisierten Abfragen

---

## 5. Cookies und lokaler Speicher

ParkHub verwendet **keine HTTP-Cookies**. Stattdessen wird der lokale Speicher des Browsers
(localStorage) fur folgende technisch notwendige Zwecke genutzt:

| Schlussel | Zweck | Inhalt | Rechtsgrundlage |
|-----------|-------|--------|-----------------|
| `parkhub_token` | Authentifizierung | Session-Token (Bearer) | SS 25 Abs. 2 Nr. 2 TTDSG |
| `parkhub_theme` | Darstellungseinstellung | `light`, `dark` oder `system` | SS 25 Abs. 2 Nr. 2 TTDSG |
| `parkhub_features` | Aktivierte Module | Liste der Modulnamen | SS 25 Abs. 2 Nr. 2 TTDSG |
| `parkhub_usecase` | Nutzungsszenario | `business`, `residential`, `personal` | SS 25 Abs. 2 Nr. 2 TTDSG |
| `parkhub_hint_*` | Onboarding-Hinweise | `1` (geschlossen) | SS 25 Abs. 2 Nr. 2 TTDSG |
| `i18nextLng` | Spracheinstellung | Sprachcode (z. B. `de`) | SS 25 Abs. 2 Nr. 2 TTDSG |

Alle Eintrage sind **technisch notwendig** (SS 25 Abs. 2 Nr. 2 TTDSG). Eine Einwilligung
ist nicht erforderlich. Es werden keine Analyse-, Werbe- oder Tracking-Cookies verwendet.

Die Progressive Web App (PWA) speichert ausschliesslich statische Dateien (JavaScript, CSS,
Schriftarten, Bilder) im Service Worker Cache. API-Antworten und Nutzerdaten werden
**niemals** im Cache gespeichert.

---

## 6. Drittanbieter-Dienste (falls aktiviert)

> **Hinweis:** Streichen Sie die folgenden Abschnitte, wenn die entsprechenden Module
> in Ihrer ParkHub-Installation nicht aktiviert sind.

### Leaflet-Kartenansicht (falls Map-Modul aktiviert)

Die Kartenansicht verwendet Leaflet mit OpenStreetMap-Kacheln. Beim Laden der Karte
werden Kartenbilder von OpenStreetMap-Servern geladen. Dabei wird Ihre IP-Adresse an
die OpenStreetMap Foundation (OSMF) ubermittelt.

- Anbieter: OpenStreetMap Foundation, St John's Innovation Centre, Cowley Road, Cambridge, CB4 0WS, UK
- Datenschutzhinweise: https://wiki.osmfoundation.org/wiki/Privacy_Policy
- Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse an Kartendarstellung)

### iCal-Kalenderabonnement (falls iCal-Modul aktiviert)

Kalenderabonnements erzeugen eine URL mit personlichem Token. Diese URL kann in externen
Kalender-Apps (Google Calendar, Apple Calendar, Outlook) eingefugt werden. Die
Kalenderdaten werden dann regelmaessig von der externen App abgerufen.

---

## 7. Kontakt fur Datenschutzanfragen

Bei Fragen zur Datenverarbeitung, Auskunftsersuchen oder Loschungsanfragen:

**[Name / Firma]**
**E-Mail:** [datenschutz@ihre-domain.de]
**Telefon:** [+49 XXX XXXXXXX]

Wir beantworten Anfragen innerhalb von 30 Kalendertagen (Art. 12 Abs. 3 DSGVO).

---

*Vorlage fur ParkHub-Betreiber (v3.2.0) -- kein Rechtsrat. Fur rechtssichere
Formulierungen empfehlen wir die Beratung durch einen Rechtsanwalt oder
Datenschutzbeauftragten.*
