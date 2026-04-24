# ParkHub v5 — Test Coverage Plan

> Mirror of parkhub-php PR #343 (T-1948). Target: 100 % happy-path + visual coverage of all 26 v5 screens shipped on `github/main` of `nash87/parkhub-rust`.

## Scope

v5 ships as a single-page Astro/React shell mounted at `/v5` via `parkhub-web/src/pages/v5.astro`. The shell reads `localStorage['ph-v5-screen']` to decide which `<ScreenComponent/>` to render and `localStorage['ph-v5-mode']` to pick the theme. Tests pin both before navigating to `/v5` so each spec targets exactly one screen in a deterministic mode.

Test base-URL resolves in this order: `E2E_BASE_URL` env → root `playwright.config.ts` default (`https://parkhub-rust-demo.onrender.com`). CI overrides to `http://127.0.0.1:18181` against `parkhub-server --headless --unattended` + `DEMO_MODE=true` (the rust analogue of PHP's `ProductionSimulationSeeder`).

## Viewports & Modes

- **Viewports** — `chromium` (Desktop Chrome, 1280 × 720 default). The `mobile-chrome` project (Pixel 5, 393 × 851) is currently **skipped** for v5 specs: the 230 px sidebar crowds the 393 px viewport, Playwright reports the <h1> as hidden, and v5 has no responsive breakpoint yet. Tracked as tech debt — mobile baselines land with the v5 responsive refactor.
- **Modes** — `marble_light` and `void`. `marble_dark` is deliberately excluded from visual baselines (large per-run AA drift on the gradient background; functional coverage provided by `dark-mode.spec.ts`).

## Visual Surfaces

`26 screens × 2 modes × 1 viewport = 52` baseline PNGs under `e2e/v5-visual.spec.ts-snapshots/`. Tolerance: `maxDiffPixelRatio: 0.02`, `fullPage: true`, `animations: 'disabled'`. Mobile-chrome baselines (+52) land with the v5 responsive refactor.

## Coverage Matrix

| # | Screen | Label (h1) | Happy-Path Anchor | Visual Surfaces |
|---|--------|------------|-------------------|-----------------|
| 01 | dashboard | Dashboard | stat-tile label "Aktive Buchungen" visible | 2 |
| 02 | buchungen | Buchungen | filter chip labelled "Alle" visible | 2 |
| 03 | buchen | Platz buchen | step indicator "Schritt 1/3" visible (wizard lands on step 1; duration chips are step-2) | 2 |
| 04 | fahrzeuge | Fahrzeuge | banner "Meine Fahrzeuge" visible (empty-state friendly) | 2 |
| 05 | kalender | Kalender | month-step button "Vorheriger Monat" present | 2 |
| 06 | karte | Karte | "Frei" summary stat OR "Keine Standorte" empty-state visible | 2 |
| 07 | credits | Credits | stat "Monatl. Kontingent" visible | 2 |
| 08 | team | Team | stat "Heute anwesend" visible | 2 |
| 09 | rangliste | Rangliste | in-screen banner "Rangliste" visible | 2 |
| 10 | ev | EV-Laden | column header "Ladepunkte" visible | 2 |
| 11 | tausch | Tausch | cta "Neue Anfrage" visible | 2 |
| 12 | einchecken | Einchecken | banner OR "Keine aktive Buchung" empty-state visible | 2 |
| 13 | vorhersagen | Vorhersagen | in-screen banner "Vorhersagen" visible | 2 |
| 14 | gaestepass | Gäste-Pass | in-screen banner "Gäste-Pass" visible | 2 |
| 15 | analytics | Analytics | in-screen banner "Analytics" visible | 2 |
| 16 | nutzer | Nutzer | in-screen banner "Nutzer" visible | 2 |
| 17 | billing | Abrechnung | banner "Abrechnung" visible | 2 |
| 18 | lobby | Lobby-Display | section-label "Aktiver Screen" OR error-card visible | 2 |
| 19 | benachrichtigungen | Benachrichtigungen | banner "Ankündigungen" visible | 2 |
| 20 | einstellungen | Einstellungen | section-label "Sprache" visible | 2 |
| 21 | standorte | Standorte | section-label "Neuer Standort" visible | 2 |
| 22 | integrations | Integrationen | banner OR error-card visible | 2 |
| 23 | apikeys | API-Schlüssel | banner OR error-card visible | 2 |
| 24 | audit | Audit-Log | in-screen banner "Audit-Log" visible | 2 |
| 25 | policies | Richtlinien | banner OR error-card visible | 2 |
| 26 | profil | Mein Profil | section-label "Kontoinformation" visible | 2 |
| — | **Total** | | **26 happy-path assertions** | **52 baselines** |

Every happy-path test also asserts the `PlaceholderV5` fallback ("Migration in Arbeit") is NOT visible — guards against accidental regression to the placeholder.

## Execution

- `npx playwright test --project=chromium e2e/v5-happy-paths.spec.ts e2e/v5-visual.spec.ts`
- CI: `.github/workflows/visual-regression.yml` runs both specs on `chromium` after `parkhub-server --headless --unattended`. `continue-on-error: true` stays on during baseline stabilisation. Rust's playwright config only registers the `chromium` project, so the `mobile-chrome` skip in the suite is a no-op on this backend.

## Baselines

Baselines are captured on Linux x64 (ubuntu-latest-compatible) via `npx playwright test --update-snapshots` run in-container or on Linux host. Committed under `e2e/v5-visual.spec.ts-snapshots/<test-name>-<project>.png`.

## Deliberate Exclusions

- `marble_dark` visual baselines — drift-prone on gradient backgrounds; covered by existing `dark-mode.spec.ts`.
- Placeholder screens — all 26 NAV entries now have real components on `github/main` (`bb7d10e` + Wave 1..4 + Admin-Nav); no placeholder surfaces remain.
- E2E-scoped interactions (CRUD, dialogs, form submissions) — out of scope for the coverage-expansion PR; followed up by later T-xxxx. This PR establishes the anchor assertions and visual baseline surfaces only.

## Risks & Mitigations

- **Font/subpixel drift between local + ubuntu-latest** — `maxDiffPixelRatio: 0.02` absorbs it. If CI drifts > 0.02 on green, raise to 0.05 (same rationale as existing `visual.spec.ts`).
- **Anti-aliased canvas in `analytics`** — UPlot canvas masked via existing `visual.spec.ts` stylesheet injection (`canvas { visibility: hidden }`). Helper re-applies the same mask.
- **Leaflet map tiles in `karte`** — same mask (`.leaflet-container { visibility: hidden }`).
- **React-query refetch-on-focus** — disabled inside helper by setting `staleTime: Infinity` via `addInitScript` patching `window.fetch` round-trip timing; fallback is `animations: 'disabled'` already inside `toHaveScreenshot`.
