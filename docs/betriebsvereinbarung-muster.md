# Betriebsvereinbarung — Digitales Parkverwaltungssystem

**Muster — rechtliche Prüfung erforderlich**
*Template — requires legal review*

---

> Dieses Dokument ist ein unverbindlicher Entwurf. Es stellt keine Rechtsberatung dar und
> ersetzt nicht die Prüfung durch eine auf Arbeits- und Betriebsverfassungsrecht spezialisierte
> Anwaltskanzlei. Vor Unterzeichnung ist eine vollständige arbeitsrechtliche Prüfung
> zwingend erforderlich.
>
> *This document is a non-binding draft. It does not constitute legal advice and does not
> replace review by a law firm specialising in labour and works-constitution law. Thorough
> legal review is mandatory before signature.*

---

## Präambel / Preamble

Zwischen

**[Arbeitgeber / Employer]**
[Unternehmensname, Anschrift / Company name, address]
— nachfolgend „Arbeitgeber" genannt —

und dem

**Betriebsrat des Betriebs [Betriebsname / Establishment name]**
— nachfolgend „Betriebsrat" genannt —

wird gemäß §§ 87 Abs. 1 Nr. 6, 75 Abs. 1 BetrVG folgende Betriebsvereinbarung
über den Einsatz des digitalen Parkverwaltungssystems ParkHub geschlossen.

*The Employer and the Works Council conclude the following works agreement pursuant to
§§ 87(1)(6) and 75(1) of the Works Constitution Act (BetrVG) regarding the use of the
digital parking management system ParkHub.*

---

## § 1 Gegenstand und Geltungsbereich / Scope

(1) Diese Betriebsvereinbarung regelt Einführung und Betrieb des digitalen
Parkverwaltungssystems ParkHub im Betrieb [Betriebsname].

(2) Das System unterstützt die Verwaltung von Parkplätzen, die Buchung von Stellplätzen
durch Mitarbeitende sowie die damit verbundene Datenverarbeitung.

(3) Diese Vereinbarung gilt für alle Mitarbeitenden, die das System nutzen oder deren
Daten im System verarbeitet werden.

*This agreement governs the introduction and operation of ParkHub at [establishment].
It applies to all employees who use the system or whose data is processed within it.*

---

## § 2 Verarbeitete Datenkategorien / Data Categories Processed

(1) Das System verarbeitet ausschließlich die in der maschinenlesbaren
Datenerhebungsübersicht (`GET /api/v1/admin/transparency/data-collection`)
aufgeführten Datenkategorien. Diese Übersicht ist jederzeit durch den Betriebsrat
abrufbar und wird bei Änderungen unverzüglich aktualisiert.

(2) Zum Zeitpunkt der Unterzeichnung sind dies insbesondere:

| Klasse / Class                | Beschreibung / Description                                    | Standard-TTL |
|-------------------------------|---------------------------------------------------------------|-------------|
| `operational_presence`        | Ein-/Auscheckvorgänge, Statusänderungen / Check-in/out events | 30 Tage     |
| `booking_history`             | Buchungsdatensätze / Booking records                          | 90 Tage     |
| `security_audit_log`          | Sicherheits- und Adminprotokoll / Security & admin audit log  | 180 Tage    |
| `hr_labour`                   | Abwesenheitsdaten / Absence records                           | 3 Jahre     |
| `anpr_raw`                    | ANPR-Rohdaten (Kennzeichen) / Raw plate reads                 | 3 Tage      |
| `ev_session`                  | Ladesitzungsdaten / EV charging session data                  | 30 Tage     |
| `billing_fiscal`              | Rechnungs-/Steuerdaten (GoBD) / Billing & fiscal records      | 8 Jahre     |

(3) Gesetzliche Mindestretenionsfristen (§ 147 AO, GoBD, §§ 87 ff. BetrVG) werden
systemseitig durchgesetzt und können nicht unterschritten werden.

*Statutory minimum retention periods are enforced by the system and cannot be undercut.*

---

## § 3 Zweck der Datenverarbeitung / Processing Purposes

Die im System verarbeiteten Daten dienen ausschließlich:

a) der Verwaltung und Zuteilung von Parkstellplätzen,
b) der Abrechnung von Stellplatznutzung,
c) der Erfüllung gesetzlicher Aufzeichnungspflichten,
d) der Betriebssicherheit (Zugangsprotokoll, ANPR),
e) dem Nachweis und der Aufklärung sicherheitsrelevanter Vorgänge.

*Data is processed exclusively for: parking allocation management, billing, statutory
record-keeping, site security, and security-incident investigation.*

---

## § 4 Zuteilungsalgorithmus und Fairness / Allocation Algorithm and Fairness

(1) Das System verwendet zur Parkplatzvergabe einen deterministischen Algorithmus
(`exact_cover_v1` und `weighted_v1`). Alle Entscheidungsparameter werden im Audit-Log
protokolliert und sind für den Betriebsrat vollständig einsehbar.

(2) **Fairness-Bericht:** Der Arbeitgeber stellt dem Betriebsrat quartalsweise einen
aggregierten Fairness-Bericht (`GET /api/v1/admin/fairness/report`) zur Verfügung.
Dieser enthält:

- Gesamtanzahl der Zuteilungen im Berichtszeitraum,
- Häufigkeitsverteilung der Zuteilungen über Nutzergruppen (k-Anonymität: k = 5),
- Gini-Koeffizient der Zuteilungsverteilung (0 = vollständige Gleichheit),
- Buchungs-zu-Zuteilungs-Verhältnis,
- kategorisierte Ablehnungsgründe.

(3) **K-Anonymität:** Kein Berichtsdatensatz enthält Informationen über Gruppen mit
weniger als 5 Personen. Individuelle Zuteilungsdaten werden dem Betriebsrat nicht
übermittelt.

(4) **Keine Leistungsbewertung:** Zuteilungsentscheidungen und -daten werden
ausdrücklich **nicht** zur Bewertung der individuellen Leistung von Mitarbeitenden
verwendet (§ 75 Abs. 1 BetrVG). Dies gilt auch für indirekte Auswertungen.

*Allocation data is expressly NOT used for individual performance evaluation of
employees (§ 75(1) BetrVG). This includes any indirect analysis.*

---

## § 5 Kein verdecktes Monitoring / No Covert Monitoring

(1) Das System führt kein verdecktes Monitoring durch. Sämtliche Datenerhebungsflächen
sind in der Datenerhebungsübersicht (§ 2) vollständig offengelegt.

(2) Eine Verhaltens- oder Leistungskontrolle der Mitarbeitenden über das System ist
unzulässig und wird technisch nicht unterstützt.

(3) Standort- oder Bewegungsprofile einzelner Mitarbeitender werden nicht erstellt.
ANPR-Rohdaten werden nach maximal 3 Tagen automatisch gelöscht.

*The system does not perform covert monitoring. Location or movement profiles of
individual employees are not created. ANPR raw data is automatically deleted after
3 days.*

---

## § 6 Rechte des Betriebsrats / Works Council Rights

(1) Der Betriebsrat erhält Lesezugriff auf folgende System-Endpunkte:

- `GET /api/v1/admin/fairness/report` — Fairness-Bericht (§ 4)
- `GET /api/v1/admin/transparency/data-collection` — Datenerhebungsübersicht (§ 2)

Hinweis: Die technische Einrichtung einer separaten `works_council`-Rolle ist in
einer Folgeversion vorgesehen. Bis dahin wird der Zugang über eine dedizierte
Admin-Kennung mit eingeschränkten Rechten gewährt.

(2) Der Betriebsrat ist vor jeder wesentlichen Änderung des Algorithmus oder der
verarbeiteten Datenkategorien gemäß § 87 Abs. 1 Nr. 6 BetrVG zu unterrichten und
anzuhören.

(3) Änderungen an Retentionsfristen bedürfen der Zustimmung des Betriebsrats, sofern
sie Auswirkungen auf die unter § 2 genannten Kategorien haben.

*The Works Council has read access to the fairness report and data-collection
disclosure endpoints. Any material change to the algorithm or data categories
requires prior consultation under § 87(1)(6) BetrVG.*

---

## § 7 Datenschutz / Data Protection

(1) Die Verarbeitung personenbezogener Daten erfolgt auf Grundlage von Art. 6 Abs. 1
lit. b DSGVO (Vertragserfüllung) und Art. 6 Abs. 1 lit. c DSGVO (rechtliche
Verpflichtung) sowie § 26 BDSG.

(2) Der Arbeitgeber trägt als Verantwortlicher im Sinne von Art. 4 Nr. 7 DSGVO die
Verantwortung für die ordnungsgemäße Datenverarbeitung.

(3) Betroffenenrechte (Auskunft, Berichtigung, Löschung) nach Art. 15–22 DSGVO bleiben
unberührt.

*Data processing is lawful under Art. 6(1)(b) and (c) GDPR and § 26 BDSG.
Data subject rights under Art. 15–22 GDPR are unaffected.*

---

## § 8 Inkrafttreten und Kündigung / Entry into Force and Termination

(1) Diese Betriebsvereinbarung tritt am [Datum / Date] in Kraft.

(2) Sie kann von jeder Seite mit einer Frist von drei Monaten zum Monatsende schriftlich
gekündigt werden. Nach Ablauf der Kündigungsfrist wirkt sie als Nachwirkung gemäß
§ 77 Abs. 6 BetrVG bis zum Abschluss einer Neuregelung fort.

*This agreement enters into force on [date] and may be terminated by either party with
three months' notice to the end of a calendar month. Aftereffect (§ 77(6) BetrVG)
applies until a replacement agreement is concluded.*

---

## Unterschriften / Signatures

Ort, Datum / Place, date: ____________________

**Für den Arbeitgeber / For the Employer:**

____________________
[Name, Funktion / Name, Title]

**Für den Betriebsrat / For the Works Council:**

____________________
[Betriebsratsvorsitzende(r) / Works Council Chair]

____________________
[Betriebsratsmitglied / Works Council Member]

---

*Dieses Muster wurde mit Hilfe des ParkHub-Systems generiert und bedarf zwingend
der rechtlichen Prüfung durch eine auf Arbeitsrecht spezialisierte Kanzlei vor
jeder Verwendung.*

*This template was generated with the assistance of the ParkHub system and requires
mandatory review by a law firm specialising in labour law before any use.*
