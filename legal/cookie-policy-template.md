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
| **Speicherdauer** | Bis zur Abmeldung oder bis das Token abläuft (24 Stunden (konfigurierbar über `session_timeout_minutes` in der Konfigurationsdatei)) |
| **Übermittlung** | Nur an die eigene ParkHub-Instanz (kein Drittanbieter) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig; Art. 6 Abs. 1 lit. b DSGVO |

### 2.2 Theme-Einstellung (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag |
| **Name** | `parkhub_theme` |
| **Inhalt** | Benutzereinstellung `light`, `dark` oder `system` |
| **Zweck** | Speichert die bevorzugte Darstellung des Nutzers |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig für gewünschte Darstellung |

### 2.3 Funktionsmodule (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag |
| **Name** | `parkhub_features` |
| **Inhalt** | Liste aktivierter Funktionsmodule (z. B. `["vehicles","credits","analytics"]`) |
| **Zweck** | Speichert, welche Funktionsbereiche der Nutzer aktiviert hat |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig für korrekte Darstellung der Benutzeroberfläche |

### 2.4 Nutzungsszenario (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag |
| **Name** | `parkhub_usecase` |
| **Inhalt** | Gewähltes Szenario: `business`, `residential` oder `personal` |
| **Zweck** | Steuert die Standardkonfiguration der Benutzeroberfläche |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig |

### 2.5 Onboarding-Hinweise (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Einträge |
| **Name** | `parkhub_hint_*` (z. B. `parkhub_hint_dashboard_intro`) |
| **Inhalt** | `1` (Hinweis wurde geschlossen) |
| **Zweck** | Verhindert wiederholtes Anzeigen bereits geschlossener Hilfehinweise |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig für gewünschtes Nutzungserlebnis |

### 2.6 Spracheinstellung (localStorage)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | localStorage-Eintrag |
| **Name** | `i18nextLng` |
| **Inhalt** | Sprachcode (z. B. `de`, `en`, `fr`) |
| **Zweck** | Speichert die vom Nutzer gewählte Sprache der Benutzeroberfläche |
| **Speicherdauer** | Dauerhaft (bis zum Löschen durch den Nutzer / Browser) |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig |

### 2.7 Service Worker / Cache API (PWA)

| Eigenschaft | Beschreibung |
|-------------|-------------|
| **Art** | Browser-Cache (Cache API, gesteuert durch Service Worker) |
| **Name** | `parkhub-v1` (Cache-Name) |
| **Inhalt** | Statische Dateien: JavaScript, CSS, Schriftarten, Bilder, SVG-Icons, SPA-Startseite |
| **Zweck** | Offline-Verfügbarkeit der Benutzeroberfläche; schnelleres Laden bei wiederholten Besuchen |
| **Gespeicherte Daten** | Ausschließlich statische, nicht-personenbezogene Anwendungsdateien. **Keine API-Antworten**, keine Nutzerdaten, keine Buchungsdaten. API-Anfragen (`/api/*`) und Health-Endpunkte werden explizit vom Caching ausgeschlossen. |
| **Speicherdauer** | Bis zum Service-Worker-Update oder manuellen Löschen der Browser-Daten |
| **Rechtsgrundlage** | § 25 Abs. 2 Nr. 2 TTDSG — technisch notwendig für PWA-Funktionalität |

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
