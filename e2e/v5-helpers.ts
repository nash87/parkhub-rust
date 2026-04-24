import type { Page } from '@playwright/test';
import { loginViaUi } from './helpers';

/**
 * v5 test helpers — T-1948.
 *
 * v5 is a single-page Astro shell mounted at `/v5/index.html`. The shell reads
 * `localStorage['ph-v5-screen']` to decide which screen component to render
 * and `localStorage['ph-v5-mode']` to pick the theme. We pin both BEFORE
 * navigating to `/v5/` so the first paint already targets the correct
 * screen + mode (avoids a dashboard → target-screen flash that would bleed
 * into visual baselines).
 *
 * Kept dependency-free beyond Playwright itself — this module is re-used by
 * both `v5-happy-paths.spec.ts` and `v5-visual.spec.ts`.
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

/** Log in through the Laravel UI as the seeded demo admin. Thin wrapper
 * so v5 specs read top-to-bottom. */
export async function loginAsAdmin(page: Page): Promise<void> {
  await loginViaUi(page);
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
  // Rust mounts v5 at `/v5` via an Astro page route (`src/pages/v5.astro`);
  // there is no `/v5/index.html` on this backend. PHP mirrored at the
  // static path — the rust equivalent is `/v5`.
  await page.goto('/v5', { waitUntil: 'domcontentloaded' });

  // Wait for the v5 shell to hydrate (h1 renders the screen label).
  await page
    .locator('header h1')
    .waitFor({ state: 'visible', timeout: 30_000 });

  // Suppress animations + hide live / chart / map surfaces. Same mask the
  // existing root visual.spec.ts uses — keeps baselines deterministic.
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
 * seedV5 is a no-op today — rust's `DEMO_MODE=true` + `--unattended` boot
 * populates the demo tenant before `parkhub-server` starts serving. Exported
 * so specs can wire extra seeding later (e.g. "team has 7 members present
 * today") without a churn-heavy import-list update.
 */
export async function seedV5(_page: Page): Promise<void> {
  // intentional no-op — DEMO_MODE has already populated the demo tenant
  // before the Playwright runner starts.
}
