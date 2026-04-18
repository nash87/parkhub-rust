import { test, expect } from '@playwright/test';
import { loginViaUi } from './helpers';

/**
 * Visual regression suite — T-1770.
 *
 * Snapshots are keyed by `{surface}-{viewport}-{theme}`. We pin viewport
 * dimensions explicitly (not via Playwright device presets) so the same
 * baseline file is reused across every project; the device-specific suffix
 * still comes from the project name Playwright appends. Running this spec
 * only under the `chromium` project keeps baselines single-variant and
 * linux-x64-pinned so CI (ubuntu-latest) and local Linux devs match.
 *
 * Tolerances:
 *   - `maxDiffPixelRatio: 0.02` = 2% — absorbs anti-aliasing jitter and
 *     font hinting without swallowing real layout regressions.
 *   - `animations: 'disabled'` — Playwright freezes CSS transitions.
 *   - We explicitly nuke animations/transitions via addStyleTag as a
 *     belt-and-braces guard.
 */

const SURFACES = [
  { name: 'login', path: '/login', auth: false },
  { name: 'dashboard', path: '/', auth: true },
  { name: 'book', path: '/book', auth: true },
  { name: 'bookings', path: '/bookings', auth: true },
  { name: 'vehicles', path: '/vehicles', auth: true },
  { name: 'admin', path: '/admin', auth: true },
  { name: 'admin-modules', path: '/admin/modules', auth: true },
];

const VIEWPORTS = [
  { name: 'desktop', width: 1440, height: 900 },
  { name: 'mobile', width: 390, height: 844 },
] as const;

const THEMES = ['light', 'dark'] as const;

for (const viewport of VIEWPORTS) {
  for (const theme of THEMES) {
    test.describe(`visual — ${viewport.name} ${theme}`, () => {
      // Only run visual suite under the default chromium project so snapshot
      // filenames stay deterministic. Mobile projects already cover their own
      // functional viewports in other specs.
      test.beforeEach(({}, testInfo) => {
        test.skip(
          testInfo.project.name !== 'chromium',
          'visual regression runs under chromium project only',
        );
      });
      test.use({ viewport: { width: viewport.width, height: viewport.height } });

      for (const surface of SURFACES) {
        test(`${surface.name}`, async ({ page }) => {
          if (surface.auth) {
            await loginViaUi(page);
          }

          // Apply theme before navigating to the target page so the first
          // render already honours it — avoids a FOUC flash that would
          // otherwise bleed into the screenshot.
          if (theme === 'dark') {
            await page.addInitScript(() => {
              try {
                localStorage.setItem('parkhub_theme', 'dark');
              } catch {
                /* quota / private mode */
              }
            });
          }

          await page.goto(surface.path, { waitUntil: 'domcontentloaded' });

          if (theme === 'dark') {
            await page.evaluate(() => {
              document.documentElement.classList.add('dark');
              document.documentElement.setAttribute('data-theme', 'dark');
            });
          }

          // Neutralise anything animated/time-dependent that could cause
          // per-run drift:
          //   - CSS animations/transitions pin at frame 0
          //   - caret blink removed
          //   - DemoOverlay (live countdown + vote counter + "Xs ago"
          //     relative timestamps) hidden — this is the production demo
          //     banner, not a real UI surface under test
          //   - ToastContainer hidden — async toasts fire-and-forget
          //   - 3rd-party chart/mapbox canvases neutralised (they render
          //     slightly differently each mount)
          await page.addStyleTag({
            content: `
              *, *::before, *::after {
                animation-duration: 0s !important;
                animation-delay: 0s !important;
                transition-duration: 0s !important;
                transition-delay: 0s !important;
                caret-color: transparent !important;
              }
              [data-demo-overlay],
              .demo-overlay,
              .Toastify,
              .Toastify__toast-container,
              [data-testid="live-counter"],
              [data-testid="timestamp"],
              .leaflet-container,
              canvas {
                visibility: hidden !important;
              }
            `,
          });

          // Wait for network-idle so lazy chunks land; then give React a
          // micro-task tick to paint the settled DOM.
          await page
            .waitForLoadState('networkidle', { timeout: 10_000 })
            .catch(() => { /* some pages stream long-poll — fall through */ });
          await page.waitForTimeout(800);

          await expect(page).toHaveScreenshot(
            `${surface.name}-${viewport.name}-${theme}.png`,
            {
              maxDiffPixelRatio: 0.02,
              fullPage: false,
              animations: 'disabled',
            },
          );
        });
      }
    });
  }
}
