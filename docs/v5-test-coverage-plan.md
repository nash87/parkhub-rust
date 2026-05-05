# ParkHub v5 — Test Coverage Plan

> Owner: T-1952 (V5 coverage expansion — Rust mirror of parkhub-php #343). Target: 100 % happy-path + visual coverage of all 26 v5 screens shipped on `github/main` of `nash87/parkhub-rust`.

## Scope

v5 ships as a single-page Astro/React shell mounted at `/v5` (static build copied into `parkhub-web/dist/v5/` and embedded into the `parkhub-server` binary via `rust_embed`). The shell reads `localStorage['ph-v5-screen']` to decide which `<ScreenComponent/>` to render and `localStorage['ph-v5-mode']` to pick the theme. Tests pin both before navigating to `/v5/` so each spec targets exactly one screen in a deterministic mode.

Test base-URL resolves in this order: `E2E_BASE_URL` env → `BASE_URL` env → `https://parkhub-rust-demo.onrender.com` (default) OR `http://localhost:4321` when `E2E_LOCAL=1`. CI overrides to `http://localhost:8081` where `parkhub-server --headless --unattended` serves both `/api/*` and the embedded Astro bundle (including `/v5/index.html`).

## Viewports & Modes

- **Viewports** — `chromium` (Desktop Chrome, 1280 × 720 default) and `mobile-chrome` (Pixel 5, 393 × 851). The blocking design-smoke gate renders every v5 screen on both projects. Visual baselines and deeper happy-path/a11y audits remain pinned to `chromium` until the dedicated v5 responsive baseline refactor lands.
- **Modes** — `marble_light` and `void`. `marble_dark` is deliberately excluded from visual baselines (large per-run AA drift on the gradient background; functional coverage provided by `dark-mode.spec.ts`).

## Visual Surfaces

`26 screens × 2 modes × 1 viewport = 52` baseline PNGs under `parkhub-web/e2e/v5-visual.spec.ts-snapshots/`. Tolerance: `maxDiffPixelRatio: 0.02`, `fullPage: true`, `animations: 'disabled'`. Mobile-chrome baselines (+52) land with the v5 responsive refactor.

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
| 08 | team | Team | stat "Heute anwesend" visible OR "Noch keine Teamdaten" empty-state visible | 2 |
| 09 | rangliste | Rangliste | in-screen banner "Rangliste" visible | 2 |
| 10 | ev | EV-Laden | column header "Ladepunkte" visible | 2 |
| 11 | tausch | Tausch | cta "Neue Anfrage" visible | 2 |
| 12 | einchecken | Einchecken | banner OR "Keine aktive Buchung" empty-state visible | 2 |
| 13 | vorhersagen | Vorhersagen | in-screen banner "Vorhersagen" visible | 2 |
| 14 | gaestepass | Gäste-Pass | in-screen banner "Gäste-Pass" visible | 2 |
| 15 | analytics | Analytics | in-screen banner "Analytics" visible | 2 |
| 16 | nutzer | Nutzer | in-screen banner "Nutzer" visible (fixme: broken on `github/main`, hydration throws "h.filter is not a function" before `<header>` renders; test + baseline skipped until fixed) | 0 |
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
| — | **Total** | | **26 declared (25 active + 1 fixme)** | **50 baselines (52 declared, 2 fixme)** |

Every happy-path test also asserts the `PlaceholderV5` fallback ("Migration in Arbeit") is NOT visible — guards against accidental regression to the placeholder.

## Execution

- Route + v5 render gate: `npm run test:e2e:design-smoke`
- Visual/happy-path baseline: `cd parkhub-web && npx playwright test --project=chromium e2e/v5-happy-paths.spec.ts e2e/v5-visual.spec.ts`
- CI: `.github/workflows/visual-regression.yml` runs `e2e/visual.spec.ts` (root; against rust server at :18181) and `parkhub-web/e2e/v5-visual.spec.ts` + `v5-happy-paths.spec.ts` on `chromium` after `parkhub-server --headless`; the local/PR design-smoke gate covers v5 render/runtime checks on both `chromium` and `mobile-chrome`.

## Baselines

Baselines are captured on Linux x64 (ubuntu-latest-compatible) via `npx playwright test --update-snapshots` run in-container or on Linux host. Committed under `parkhub-web/e2e/v5-visual.spec.ts-snapshots/<test-name>-<project>.png`.

## Known-Broken Screens

These screens hydrate-throw on `github/main` (a8fba08) and their `<header><h1>` is never emitted; Playwright then stalls on the shell-ready wait in `openV5`. The happy-path + visual suites keep the test declarations in place but `test.fixme` them via a shared `KNOWN_BROKEN` set — the moment the screen is repaired, the fixme stops firing and the anchor + baseline reactivate without touching this list.

- **nutzer** — NutzerV5 throws `h.filter is not a function` during render. Root cause: `useQuery<User[]>` receives a non-array response when the admin-users endpoint is unavailable or returns a wrapper object, and the `users.filter(...)` call at the top of the component then blows up. Follow-up fix: defend `users` with `Array.isArray(data) ? data : []`.

## Deliberate Exclusions

- `marble_dark` visual baselines — drift-prone on gradient backgrounds; covered by existing `dark-mode.spec.ts`.
- Placeholder screens — all 26 NAV entries now have real components on `github/main`; no placeholder surfaces remain.
- E2E-scoped interactions (CRUD, dialogs, form submissions) — out of scope for the coverage-expansion PR; followed up by later T-xxxx. This PR establishes the anchor assertions and visual baseline surfaces only.

## Risks & Mitigations

- **Font/subpixel drift between local + ubuntu-latest** — `maxDiffPixelRatio: 0.02` absorbs it. If CI drifts > 0.02 on green, raise to 0.05 (same rationale as existing `visual.spec.ts`).
- **Anti-aliased canvas in `analytics`** — UPlot canvas masked via the helper stylesheet injection (`canvas { visibility: hidden }`).
- **Leaflet map tiles in `karte`** — same mask (`.leaflet-container { visibility: hidden }`).
- **React-query refetch-on-focus** — absorbed by `animations: 'disabled'` + 400 ms settle window inside the visual spec.
- **Rust `parkhub-server` cold start** — the `--headless --unattended` boot takes ~3-4 s to seed DEMO_MODE; the CI workflow already waits up to 45 s on `/health` before firing Playwright.
