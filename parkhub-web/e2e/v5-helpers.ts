import type { Page } from '@playwright/test';

/**
 * v5 test helpers — Rust mirror of PHP #343 coverage expansion (T-1952).
 *
 * v5 is a single-page Astro shell mounted at `/v5/index.html` (the Astro
 * page at `src/pages/v5.astro` renders `V5App` into `#root`). The shell
 * reads `localStorage['ph-v5-screen']` to decide which screen component
 * to render and `localStorage['ph-v5-mode']` to pick the theme. We pin
 * both BEFORE navigating to `/v5/` so the first paint already targets
 * the correct screen + mode (avoids a dashboard → target-screen flash
 * that would bleed into visual baselines).
 *
 * Kept dependency-free beyond Playwright itself — this module is re-used
 * by both `v5-happy-paths.spec.ts` and `v5-visual.spec.ts`.
 *
 * Byte-compatible with `parkhub-php` `e2e/v5-helpers.ts` on the storage
 * keys, screen ids, and labels; the only adaptations are the login flow
 * (`#demo-autofill` button on the Rust `/login` page instead of Laravel
 * form POST) and the fact that the Rust `parkhub-server` serves the
 * Astro bundle itself — no separate dev server.
 */

/** Canonical v5 screen ids, in the order they appear in the sidebar. */
export const V5_SCREENS = [
  'dashboard',
  'buchungen',
  'buchen',
  'fahrzeuge',
  'kalender',
  'karte',
  'credits',
  'team',
  'rangliste',
  'ev',
  'tausch',
  'einchecken',
  'vorhersagen',
  'gaestepass',
  'analytics',
  'nutzer',
  'billing',
  'lobby',
  'benachrichtigungen',
  'einstellungen',
  'standorte',
  'integrations',
  'apikeys',
  'audit',
  'policies',
  'profil',
] as const;

export type V5Screen = typeof V5_SCREENS[number];

/** h1 title emitted by <V5TopBar> for each screen, keyed by screen id. */
export const V5_LABELS: Record<V5Screen, string> = {
  dashboard: 'Dashboard',
  buchungen: 'Buchungen',
  buchen: 'Platz buchen',
  fahrzeuge: 'Fahrzeuge',
  kalender: 'Kalender',
  karte: 'Karte',
  credits: 'Credits',
  team: 'Team',
  rangliste: 'Rangliste',
  ev: 'EV-Laden',
  tausch: 'Tausch',
  einchecken: 'Einchecken',
  vorhersagen: 'Vorhersagen',
  gaestepass: 'Gäste-Pass',
  analytics: 'Analytics',
  nutzer: 'Nutzer',
  billing: 'Abrechnung',
  lobby: 'Lobby-Display',
  benachrichtigungen: 'Benachrichtigungen',
  einstellungen: 'Einstellungen',
  standorte: 'Standorte',
  integrations: 'Integrationen',
  apikeys: 'API-Schlüssel',
  audit: 'Audit-Log',
  policies: 'Richtlinien',
  profil: 'Mein Profil',
};

/** v5 theme modes we exercise. `marble_dark` is deliberately excluded
 * from visual baselines — large per-run AA drift on the gradient bg. */
export const V5_MODES = ['marble_light', 'void'] as const;
export type V5Mode = typeof V5_MODES[number];

/**
 * Log in through the Rust UI as the seeded demo admin. The Rust backend
 * exposes `admin@parkhub.test` / `demo` when DEMO_MODE=true, and the
 * login page renders a `#demo-autofill` button that fills the form for
 * us (see `parkhub-web/e2e/login.spec.ts`).
 *
 * PHP ships `loginViaUi` in `e2e/helpers.ts`; the Rust repo has no
 * equivalent module, so the login dance is inlined here.
 */
export async function loginAsAdmin(page: Page): Promise<void> {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  // Post-login, the dashboard renders "Active Bookings" (English) as its
  // lead stat card — same sentinel used by `dashboard.spec.ts`.
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

export function v5ScreenTitle(page: Page) {
  return page.getByTestId('v5-screen-title');
}

/**
 * Pre-seed localStorage so the v5 shell boots already on the target
 * screen + mode, then navigate to `/v5/index.html` and wait for the
 * first paint to settle.
 */
export async function openV5(
  page: Page,
  screen: V5Screen,
  mode: V5Mode = 'marble_light',
): Promise<void> {
  await page.addInitScript(
    ([s, m]) => {
      try {
        window.localStorage.setItem('ph-v5-screen', s);
        window.localStorage.setItem('ph-v5-mode', m);
      } catch {
        // Private-mode / quota — first-paint falls back to dashboard, tests
        // then detect the wrong screen and fail fast. Preferable to a
        // silently-wrong baseline.
      }
    },
    [screen, mode] as const,
  );
  await page.goto('/v5/index.html', { waitUntil: 'domcontentloaded' });

  // Wait for the v5 shell to hydrate (h1 renders the screen label).
  await v5ScreenTitle(page).waitFor({ state: 'visible', timeout: 30_000 });

  // Suppress animations + hide live / chart / map surfaces. Same mask the
  // existing root `e2e/visual.spec.ts` uses — keeps baselines deterministic.
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

  // Let react-query settle any prefetches so the baseline shows real data,
  // not skeletons. Network-idle is best-effort — some endpoints long-poll.
  await page
    .waitForLoadState('networkidle', { timeout: 8_000 })
    .catch(() => {
      /* fall through — toHaveScreenshot has its own retry budget */
    });
}

/**
 * seedV5 is a no-op today — the Rust `parkhub-server` runs with
 * DEMO_MODE=true in CI, which provisions the demo tenant + admin user
 * during `--headless --unattended` startup. Exported so specs can wire
 * extra seeding later (e.g. "team has 7 members present today") without
 * a churn-heavy import-list update.
 */
export async function seedV5(_page: Page): Promise<void> {
  // intentional no-op — parkhub-server seeds the demo tenant on boot.
}
