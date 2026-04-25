# parkhub-web (Rust runtime)

Frontend workspace for the **Rust edition** of ParkHub. Astro 6 ships a static SPA shell;
React 19 + Tailwind CSS 4 render the application; the production build is embedded into
the `parkhub-server` binary and served by Axum as static files.

> Top-level project docs (product overview, deployment, API, compliance) live in the
> repository root: [`../README.md`](../README.md), [`../DEVELOPMENT.md`](../DEVELOPMENT.md),
> [`../ARCHITECTURE.md`](../ARCHITECTURE.md), [`../CHANGELOG.md`](../CHANGELOG.md).

## Stack

| Layer            | Technology                                                  |
|------------------|-------------------------------------------------------------|
| Framework        | [Astro](https://astro.build/) 6.1 (`output: 'static'`)      |
| UI runtime       | [React](https://react.dev/) 19.2 + React Compiler (Babel)   |
| Styling          | [Tailwind CSS](https://tailwindcss.com/) 4.2 via Vite plugin |
| Routing          | `react-router-dom` 7                                        |
| Data             | TanStack Query 5 + TanStack Table 8                         |
| Forms            | `react-hook-form` 7 + `zod` 4 + `@hookform/resolvers` 5     |
| Charts           | uPlot 1.6 · Maps: Leaflet 1.9 + `react-leaflet` 5           |
| i18n             | `i18next` 26 + `react-i18next` 17, 10 locales, hot-loaded   |
| Command Palette  | `cmdk` 1.1 (mounted globally, `Cmd+K` / `Ctrl+K`)           |
| Types            | `ts-rs`-generated bindings in `src/generated/types/`        |
| Lint / Format    | Biome 2                                                     |
| Node             | `>= 22.12`                                                  |

## Layout — how it embeds in the Rust binary

```
parkhub-web/                  # this workspace
├── src/                      # React + Astro source
├── dist/                     # `astro build` output (gitignored)
└── ...
parkhub-server/
└── src/                      # Axum app uses `rust-embed` to bake dist/ into the binary
```

`npm run build:rust` produces `parkhub-web/dist/` and the `parkhub-server` build embeds
those assets via `rust-embed`, so a single `parkhub-server` binary ships the full SPA.
Axum serves them as static files alongside `/api/v1/*`. No nginx, no separate frontend
deploy — see [`../ARCHITECTURE.md`](../ARCHITECTURE.md) for the binary diagram.

## Scripts

| Script                  | Action                                                          |
|-------------------------|-----------------------------------------------------------------|
| `npm run dev`           | Astro dev server on `http://localhost:4321`                     |
| `npm run build`         | Production build into `dist/`                                   |
| `npm run build:rust`    | Build + sync `dist/` into the Rust workspace for `rust-embed`   |
| `npm run preview`       | Preview the production build locally                            |
| `npm run test`          | Vitest unit + component tests (jsdom, single run)               |
| `npm run test:watch`    | Vitest in watch mode                                            |
| `npm run test:coverage` | Vitest with v8 coverage (40 % statements gate)                  |
| `npm run test:e2e`      | Playwright against `E2E_BASE_URL` (defaults to live demo)       |
| `npm run test:e2e:local`| Hermetic local run via `scripts/e2e-local.sh` (Astro + server)  |
| `npm run test:a11y`     | Playwright + `@axe-core/playwright` accessibility spec          |
| `npm run lint`          | Biome check on `src/`                                           |
| `npm run lint:fix`      | Biome check + autofix                                           |
| `npm run i18n:coverage` | Locale-key coverage report                                      |

## Testing strategy

- **Vitest 4** (`vitest.config.ts`) — jsdom env, Testing Library (`@testing-library/react`,
  `@testing-library/user-event`), v8 coverage with hard thresholds (40 % statements,
  30 % branches, 35 % functions, 40 % lines).
- **Playwright 1.59** (`playwright.config.ts`) — three run modes: live demo (default),
  CI-local against a running server, and hermetic dev-local (`E2E_LOCAL=1` boots Astro
  on `:4321` and proxies `/api` to a local `parkhub-server`). `chromium` + `mobile-chrome`
  (Pixel 5) projects.
- **Axe-core a11y** — `@axe-core/playwright` runs on every Design v5 route; `npm run test:a11y`
  exercises the dedicated accessibility spec.
- **Lighthouse CI** — `lighthouserc.json` enforces a11y ≥ 95, performance ≥ 90, SEO ≥ 95
  on every PR (see top-level [`../README.md`](../README.md#-testing)).

## Related docs

- [`../README.md`](../README.md) — product overview, deployment, screenshots
- [`../DEVELOPMENT.md`](../DEVELOPMENT.md) — local dev loop, `make ci`, pre-commit hooks
- [`../ARCHITECTURE.md`](../ARCHITECTURE.md) — backend + embedding architecture
- [`../CHANGELOG.md`](../CHANGELOG.md) — release notes
- [`../docs/openapi/rust.json`](../docs/openapi/rust.json) — REST contract consumed by this UI
