# EU AI Act — Transparenz­erklärung für KI-Funktionen in ParkHub

> **Anleitung für Betreiber:** Diese Vorlage wird **relevant, sobald eine
> KI-Funktion aktiviert wird** (z. B. das geplante Modul *AI Occupancy
> Forecast*, `MODULE_AI_FORECAST`, siehe Roadmap). Ohne aktivierte KI-Module
> gelten die Transparenz­pflichten dieses Dokuments nicht.
>
> Füllen Sie die Platzhalter entsprechend der bei Ihnen aktivierten Module aus
> und veröffentlichen Sie die Erklärung unter `/ki-transparenz` (oder
> verlinken Sie sie aus Ihrer Datenschutz­erklärung).
>
> **Rechtliche Grundlage:**
> - **VO (EU) 2024/1689 (AI Act)** — Kapitel IV, insbesondere:
>   - Art. 50 Abs. 1 — Transparenz­pflichten bei KI-Systemen mit direktem
>     Nutzer­kontakt (anwendbar seit 2. August 2026).
>   - Art. 50 Abs. 4 — Kennzeichnung KI-generierter Inhalte.
>   - Art. 6 Anhang III — Hochrisiko-Einstufung (aktuell **nicht** einschlägig
>     für die in ParkHub geplanten Funktionen, siehe Abschnitt 2).
> - **DSGVO Art. 22** — Ergänzend bei automatisierten Entscheidungen.

---

## 1. Aktivierte KI-Funktionen

| Modul | Status | Zweck | Datenbasis |
|-------|--------|-------|------------|
| `ai_forecast` (Occupancy Forecast) | **[geplant / aktiv / deaktiviert]** | Prognose der Stellplatz­auslastung der nächsten 7 Tage pro Lot | Anonymisierte historische Buchungs­aggregate der letzten 180 Tage |
| `ai_narrative` (Dashboard-Text) | **[geplant / aktiv / deaktiviert]** | Natur­sprachliche Zusammen­fassung der Dashboard-KPIs | Nur serverseitig berechnete Aggregate, keine personen­bezogenen Daten |
| _weitere Module_ | | | |

## 2. Risiko-Einstufung nach AI Act

- Die geplanten Module sind **keine Hochrisiko-Systeme** im Sinne von
  Art. 6 AI Act und Anhang III. Sie entscheiden **nicht** über Zugang zu
  wesentlichen Dienst­leistungen, nicht über Beschäftigung und nicht über
  Kreditwürdigkeit.
- Die Module sind **Systeme mit beschränktem Risiko** (limited risk). Für sie
  gelten ausschließlich die **Transparenz­pflichten** aus Art. 50 Abs. 1 und 4,
  nicht aber die Konformitäts­bewertung und CE-Kennzeichnung aus Art. 43.
- Ergebnisse sind **Entscheidungs­hilfen für Administrator:innen**, nicht
  autonome Entscheidungen — ein Mensch bestätigt jede Handlung, bevor sie
  Wirkung entfaltet.

## 3. Transparenz­hinweise an Nutzer (Art. 50 Abs. 1)

- In der ParkHub-Oberfläche erscheint bei jeder KI-generierten Ausgabe ein
  sichtbares Label **„🤖 KI-generiert"** inklusive Tool­tip, der Zweck,
  Modell und Konfidenz­bereich erklärt.
- Prognose­werte werden **immer mit Unsicherheits­intervall** (z. B.
  „68 % ± 12 %") dargestellt — niemals als Einzel­punkt, der Gewissheit
  suggeriert.
- **Opt-out:** Nutzer:innen können KI-Ausgaben pro Modul unter
  **Admin → Einstellungen → KI-Funktionen** global deaktivieren; die
  klassische regel­basierte Auswertung bleibt verfügbar.

## 4. Eingesetzte Modelle und Datenbasis

### 4.1 Occupancy Forecast

- **Verfahren:** Klassisches zeitreihen­basiertes Modell
  (**SARIMA (2,1,2)(1,0,1,7)**), lokal auf dem ParkHub-Server gerechnet, kein
  externer API-Call, keine Cloud-Verbindung.
- **Trainings­daten:** Die eigenen Buchungs­aggregate der letzten 180 Tage,
  stündlich gerastert, ohne Personen­bezug (kein `user_id`, kein Kennzeichen,
  kein Vertrags­name).
- **Aktualisierung:** Tägliches Re-Fitting um 03:00 Uhr UTC.
- **Erklärbar­keit:** Saison-, Trend- und Residual-Komponenten sind im
  Dashboard separat auslesbar.

### 4.2 Dashboard-Narrativ (optional)

- **Verfahren:** **Lokales Sprach­modell** (Gemma 3 4B oder gleichwertig) auf
  dem ParkHub-Server oder auf einer dedizierten Inferenz­maschine im selben
  Netz. **Kein externer LLM-Provider.**
- **Eingabe:** Ausschließlich numerische Aggregate (Auslastung, Umsätze,
  Booking-Counts), keine Namen, keine Kennzeichen.
- **Ausgabe:** 2–3 Sätze natürliche Sprache.
- **Guard­rails:** System-Prompt erzwingt Deutsch/Englisch-Switch anhand
  `Accept-Language`; keine Halluzination von Zahlen außerhalb der Eingabe.

## 5. Daten­schutz und Datenminimierung (DSGVO-Bezug)

- Für die KI-Funktionen werden **keine personen­bezogenen Daten** verarbeitet.
  Aggregate werden serverseitig anonym gebildet, bevor das Modell sie sieht.
- Rechts­grundlage gemäß DSGVO ist daher **nicht relevant**, weil keine
  Verarbeitung personen­bezogener Daten stattfindet. Sollte dies durch
  zukünftige Module anders werden, wird diese Erklärung aktualisiert, die
  Rechts­grundlage ergänzt und die Datenschutz­erklärung angepasst.

## 6. Grenzen und Verantwortungs­bereich

- Prognosen basieren auf historischen Mustern und können **neue oder
  außer­gewöhnliche Ereignisse** (Streik, Sperrung, Großveranstaltung) nicht
  vorhersagen.
- ParkHub liefert Entscheidungs­hilfen, keine Garantien — Betreiber bleibt
  vollständig verantwortlich für jede darauf aufbauende Handlung.
- Konfidenz­intervalle werden immer mit ausgegeben; Werte unter 50 %
  Konfidenz werden in der UI rot markiert und nicht für Automatik-Regeln
  verwendet.

## 7. Logging und Audit (Art. 12 AI Act, antizipierte Hochrisiko-Erweiterung)

Auch wenn die aktuellen Module keine Hochrisiko-Systeme sind, loggen wir
präventiv:

- Zeitstempel jeder Vorhersage,
- Eingabe-Hash (SHA-256 der Aggregate),
- Modell­version und -konfiguration,
- Ausgabe + Konfidenz­intervall,
- Nutzer-/Tenant-Scope (aggregiert, ohne Personen­bezug).

Aufbewahrung: **90 Tage** (überschreibt das allgemeine Audit-Log-Limit nicht).

## 8. Änderungs­historie

| Datum | Änderung |
|-------|----------|
| [JJJJ-MM-TT] | Erst­fassung |

---

## 9. Kontakt für Fragen zur KI-Nutzung

- **AI-Governance-Verantwortliche:r:** [Name, E-Mail]
- **DSGVO-Kontakt:** [aus Datenschutz­erklärung übernehmen]
- **Meldung von Fehlfunktionen / Bias-Verdacht:** [ai-feedback@ihre-domain.de]
