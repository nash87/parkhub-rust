# Cookie-Richtlinie / Cookie Policy — ParkHub

> **Anleitung für Betreiber:** Diese Cookie-Richtlinie können Sie direkt verwenden oder in Ihre
> Datenschutzerklärung integrieren. ParkHub verwendet keine Tracking-Cookies — dieser Text
> spiegelt den tatsächlichen Stand der Software wider.
>
> **Rechtliche Grundlage:** § 25 TTDSG (Telekommunikation-Telemedien-Datenschutz-Gesetz)
> in Verbindung mit Art. 5 Abs. 3 der ePrivacy-Richtlinie (2002/58/EG).

---

# Cookie-Richtlinie

**[Name des Betreibers]**
**Stand:** [Datum]

---

## 1. Überblick

ParkHub ist ein On-Premise-Parkplatzmanagementsystem. Diese Website / Anwendung verwendet
**ausschließlich technisch notwendige Speicherformen** (localStorage). Es werden **keine
Tracking-Cookies, keine Analyse-Cookies und keine Werbe-Cookies** eingesetzt.

---

## 2. Welche Speichertechnologien wir verwenden

### 2.1 Authentication Token (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag (kein HTTP-Cookie) |
| **Name** | `parkhub_token` (oder `token`) |
| **Inhalt** | JWT-Authentifizierungstoken (enthält Benutzer-ID und Ablaufzeit) |
| **Zweck** | Technisch notwendig — ermöglicht eingeloggten Zugriff auf die Anwendung |
| **Speicherdauer** | Bis zur Abmeldung oder bis das Token abläuft (typisch 7 Tage) |
| **Übermittlung** | Nur an die eigene ParkHub-Instanz (kein Drittanbieter) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig; Art. 6 Abs. 1 lit. b DSGVO |

### 2.2 Theme-Einstellung (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag |
| **Name** | `parkhub_theme` (o. ä.) |
| **Inhalt** | Benutzereinstellung „hell" oder „dunkel" |
| **Zweck** | Speichert die bevorzugte Darstellung des Nutzers |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig für gewünschte Darstellung |

---

## 3. Keine Cookies von Drittanbietern

ParkHub lädt **keine externen Ressourcen** (kein Google Analytics, kein Facebook Pixel,
kein CDN-JavaScript von Dritten, keine eingebetteten Videos, keine Social-Media-Buttons).
Daher werden keinerlei Third-Party-Cookies gesetzt.

---

## 4. Keine Tracking- oder Profilbildung

ParkHub erstellt **kein Nutzungsprofil** und führt **keine verhaltensbasierte Auswertung**
durch. Die einzigen gespeicherten Daten sind Buchungen und Fahrzeugdaten, die zur
Vertragserfüllung notwendig sind (Art. 6 Abs. 1 lit. b DSGVO).

---

## 5. Verwaltung und Löschung von localStorage-Einträgen

Da ParkHub localStorage statt HTTP-Cookies verwendet, können Sie die Daten wie folgt löschen:

**In allen gängigen Browsern:**
1. Entwicklertools öffnen (F12)
2. Reiter „Anwendung" (Chrome/Edge) oder „Speicher" (Firefox)
3. Unter „Lokaler Speicher" → Domain des ParkHub-Servers auswählen
4. Einträge einzeln löschen oder „Alle löschen"

**Oder:** Einfach auf „Abmelden" klicken — der Token wird dann aus dem localStorage entfernt.

---

## 6. Zukünftige Änderungen

Sollte eine zukünftige Version von ParkHub Cookies oder zusätzliche Speichertechnologien
einführen, wird diese Richtlinie entsprechend aktualisiert. Das Datum des letzten Updates
ist oben angegeben.

---

## 7. Kontakt

Bei Fragen zu dieser Cookie-Richtlinie wenden Sie sich an:

**[Name des Verantwortlichen]**
**E-Mail:** [datenschutz@ihre-domain.de]

Weitere Informationen zur Datenverarbeitung finden Sie in unserer
[Datenschutzerklärung](/datenschutz).

---

*Vorlage für ParkHub-Betreiber — kein Rechtsrat. Diese Vorlage spiegelt die Standard-
Konfiguration von ParkHub wider. Abweichende Konfigurationen (z. B. externe SMTP-Dienste,
Monitoring-Tools) müssen gesondert dokumentiert werden.*
