# E2E test suite

Playwright + axe-core. 2026 stack: hermetic local runs via Astro dev-server + release-build `parkhub-server`, optional MCP bridge for AI-driven vibe-coding loops.

## Run modes

| Mode                  | Command                              | Target                                                   |
| --------------------- | ------------------------------------ | -------------------------------------------------------- |
| Cloud (default)       | `npm run test:e2e`                   | `https://parkhub-rust-demo.onrender.com`                 |
| CI-local              | `E2E_BASE_URL=http://localhost:8081 npm run test:e2e` | Pre-running server on 8081                  |
| Hermetic-local        | `npm run test:e2e:local`             | Builds server + starts Astro dev + runs suite end-to-end |
| Playwright UI         | `npm run test:e2e:ui`                | Time-travel debug, watch mode, trace viewer              |
| Headed                | `npm run test:e2e:headed`            | Same as hermetic but visible Chromium                    |
| Design surfaces only  | `npm run test:e2e:design`            | `e2e/design-*.spec.ts`                                   |
| A11y only             | `npm run test:a11y`                  | axe-core WCAG 2.1 AA pass over post-login surfaces       |

## Layout

- `fixtures/axe.ts` — Playwright fixture exposing an `axe` helper that runs
  `@axe-core/playwright` with WCAG 2.1 AA + best-practice tags, attaches the
  JSON report to the trace, and asserts zero serious/critical violations.
- `design-*.spec.ts` — covers the claude.ai/design integration (#335):
  Assistant, ShortcutsHelp, Settings, plus a broad accessibility pass.
- `*.spec.ts` — everything else (bookings, admin, PWA, etc.).

## AI-driven flows (Playwright MCP)

[`@playwright/mcp`](https://github.com/microsoft/playwright-mcp) exposes the
running browser to Claude Code or any MCP-compatible agent. Two options:

### One-shot, live against the hermetic stack

```bash
# Terminal 1
npm run test:e2e:ui             # starts server + Astro + UI mode

# Terminal 2 — hand the running Chromium to Claude
npx @playwright/mcp@latest --port 7700
```

Then add to `.mcp.json`:

```json
{ "mcpServers": { "playwright": { "url": "http://localhost:7700/mcp" } } }
```

Claude can now click, type, and assert on the running page while you
iterate — "click the assistant icon, dictate a new booking, screenshot the
result" becomes a single prompt.

### Per-test recording

```bash
npx playwright codegen http://localhost:4321   # records a new spec from clicks
```

## Writing a new design spec

```ts
import { test, expect } from './fixtures/axe';

test('my new dialog is keyboard-accessible', async ({ page, axe }) => {
  await page.goto('/some-route');
  await page.keyboard.press('Control+.');
  await expect(page.getByRole('dialog')).toBeVisible();
  await axe({ include: '[role="dialog"]' });  // asserts zero serious a11y violations
});
```

## Reports

- `test-results/` — traces, screenshots, videos (retained on failure)
- `playwright-report/` — HTML report (auto-opens after local non-CI runs)
- Axe JSON reports are attached to each failing test as `axe-report.json`
