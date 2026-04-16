# Erklärung zur Barrierefreiheit — ParkHub

> **Anleitung für Betreiber:** Diese Erklärung ist seit dem **28. Juni 2025**
> nach § 14 BFSG (Barrierefreiheitsstärkungsgesetz) verpflichtend für jedes
> B2C-Angebot mit ≥ 10 Beschäftigten oder ≥ 2 Mio. € Jahresumsatz, das in
> Deutschland an Verbraucher gerichtet ist. Füllen Sie die Platzhalter aus und
> veröffentlichen Sie die fertige Version unter `/barrierefreiheit` (oder
> verlinken Sie sie im Footer).
>
> **Rechtliche Grundlage:**
> - § 14 BFSG (BGBl. I 2021 Nr. 39) — Pflicht zur Barrierefreiheitserklärung
> - EN 301 549 V3.2.1 — harmonisierte europäische Norm
> - WCAG 2.1 AA — technischer Maßstab für Webangebote
> - BITV 2.0 — für öffentliche Stellen (zusätzlich)

---

## 1. Anwendungsbereich

Diese Erklärung gilt für die unter `[https://parkhub.example.com]` veröffentlichte
ParkHub-Instanz von **[Firmenname]** einschließlich aller Unter­seiten,
des Buchungs­dashboards und der Progressive-Web-App-Variante.

## 2. Vereinbarkeitsstatus

ParkHub ist mit den Anforderungen der WCAG 2.1 Level AA **weit­gehend
vereinbar**. Die unter Abschnitt 4 aufgeführten Ausnahmen sind bekannt und
werden adressiert.

## 3. Umgesetzte Barrierefreiheit

- **Tastatur­bedienbarkeit:** Alle interaktiven Elemente sind per Tab
  erreichbar, der sichtbare Fokus­ring hebt das aktive Element deutlich hervor.
- **Semantisches HTML:** Überschriften­hierarchie (h1–h4), Landmark-Rollen
  (`<nav>`, `<main>`, `<aside>`) und Formular­labels (`<label for=…>`) sind
  durch­gehend gesetzt.
- **ARIA-Rollen:** Alle icon-only Buttons tragen `aria-label`, Modals fangen
  den Fokus, Tabs verwenden `role="tablist" / role="tab" / role="tabpanel"`.
- **Kontrast:** Die Tailwind-Palette erfüllt mindestens das Kontrast­verhältnis
  4,5 : 1 für Fließtext und 3 : 1 für große Schrift.
- **Responsive Design:** Zoom bis 400 % ohne horizontales Scrollen; mobile
  Sichten ab 320 px Breite; iOS safe-area-Respektierung.
- **Reduzierte Bewegung:** `prefers-reduced-motion` wird respektiert —
  Animationen (Framer Motion, View-Transitions) schalten sich automatisch ab.
- **Sprachumschaltung:** Zehn Sprachen (DE/EN/FR/ES/IT/PT/TR/PL/JA/ZH) über
  automatische Erkennung und manuelle Auswahl.
- **Accessible Parking Feature:** Dedizierte Buchungs­kategorie mit
  Anforderungen `wheelchair_access`, `extra_width`, `close_to_entrance` —
  Buchungs­filter zeigt nur geeignete Stell­plätze.
- **Barrierearme E-Mails:** Buchungs­bestätigungen sind reiner HTML-Text mit
  inline-Tabellen, keine Bilder-mit-Text, kein JavaScript.

## 4. Bekannte Einschränkungen

- **[Bitte ergänzen:] ____________**
  *(Beispiele: einzelne Diagramme im Admin-Dashboard sind ausschließlich
  visuell und benötigen noch eine Tabellen­alternative; PDF-Exporte von
  Rechnungen sind nicht vollständig getaggt; die Karten­ansicht der Lots ist
  bisher nur mit Maus bedienbar.)*

Ein Maßnahmen­plan zur Beseitigung dieser Einschränkungen liegt vor; Ziel­termin:
**[JJJJ-MM-TT]**.

## 5. Feedback und Beschwerde­weg

### Feedback an den Betreiber

Wenn Sie barriere­frei­heits­bezogene Mängel feststellen oder Inhalte in
anderer Form benötigen, kontaktieren Sie uns:

- **E-Mail:** [accessibility@ihre-domain.de]
- **Postweg:** [Name, Straße, PLZ Ort]
- **Telefon / Textmessage:** [+49 XXX XXXXXXX]

Wir antworten spätestens **innerhalb von 6 Wochen** und geben den Stand der
Abhilfe­maßnahmen an.

### Schlichtungs­verfahren

Wenn auf Ihre Rückmeldung keine zufriedenstellende Lösung folgt, können Sie
sich an die **Schlichtungsstelle BGG** (Schlichtungsstelle nach dem
Behindertengleichstellungsgesetz) wenden:

- **Schlichtungsstelle BGG**
  Mauerstraße 53
  10117 Berlin
- **Telefon:** +49 30 185 271 805
- **E-Mail:** info@schlichtungsstelle-bgg.de
- **Web:** https://www.schlichtungsstelle-bgg.de

### Durchsetzungs­stelle (Marktüberwachung)

Die zuständige Marktüberwachungs­behörde nach § 19 BFSG in Deutschland ist
die **Bundesfachstelle Barrierefreiheit**:

- **Bundesfachstelle Barrierefreiheit** bei der KfW
  Charlottenstraße 59
  10117 Berlin
- **E-Mail:** info@bundesfachstelle-barrierefreiheit.de
- **Web:** https://www.bundesfachstelle-barrierefreiheit.de

## 6. Technische Details

- **Evaluations­methode:** Selbstbewertung kombiniert mit automatisierter
  Prüfung via **axe-core** (Playwright-Integration, CI-Gate) sowie manuellen
  Tests mit Tastatur-only und NVDA 2024.x unter Windows 11.
- **Zuletzt aktualisiert:** [JJJJ-MM-TT]
- **Nächste Prüfung:** spätestens **12 Monate** nach Veröffentlichung.

## 7. Fassung dieser Erklärung

Diese Erklärung wurde am **[JJJJ-MM-TT]** nach dem in Anhang III der Durch­führungs­beschluss-EU-2018/1523 vorgegebenen Muster erstellt.
