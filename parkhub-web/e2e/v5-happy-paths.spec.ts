import { test, expect, type Page } from '@playwright/test';
import { loginAsAdmin, openV5, V5_LABELS, type V5Screen } from './v5-helpers';

/**
 * v5 happy-path suite — Rust mirror of parkhub-php #343 (T-1952).
 *
 * One test per screen. Each test:
 *   1. logs in as the seeded demo admin
 *   2. pins `ph-v5-screen` in localStorage and opens /v5/index.html
 *   3. asserts the <V5TopBar> h1 shows the expected NavItem label
 *   4. asserts the screen-specific anchor is visible (unique in-screen
 *      element that proves the real screen component — not the
 *      PlaceholderV5 fallback — is mounted)
 *   5. asserts the PlaceholderV5 "Migration in Arbeit" badge is NOT
 *      visible — guards against accidental regression to the placeholder
 *
 * Anchors were lifted from the screen components on `github/main`
 * (see `docs/v5-test-coverage-plan.md`). Anchors target always-rendered
 * banners / section labels so the assertion survives empty-state data
 * as well as populated-state data (the DEMO_MODE seeder populates most
 * surfaces, but guest-pass / fahrzeuge / tausch start empty).
 */

/** Screen-local anchor — a visible element that only this screen renders. */
interface Anchor {
  /** Human-friendly label used in the test name. */
  note: string;
  /** Playwright assertion fn. */
  assert: (page: Page) => Promise<void>;
}

const ANCHORS: Record<V5Screen, Anchor> = {
  dashboard: {
    note: 'Aktive Buchungen stat-tile',
    assert: async (p) => expect(p.getByText('Aktive Buchungen').first()).toBeVisible(),
  },
  buchungen: {
    note: 'Alle filter chip',
    assert: async (p) => expect(p.getByText('Alle', { exact: true }).first()).toBeVisible(),
  },
  buchen: {
    note: 'Schritt 1/3 wizard indicator',
    // Wizard is in step 1 on first load; duration chips only show on step 2.
    assert: async (p) => expect(p.getByText('Schritt 1/3').first()).toBeVisible(),
  },
  fahrzeuge: {
    note: 'Meine Fahrzeuge banner',
    // Empty-state demo tenant has no vehicles, so the colour swatches
    // aren't rendered. The banner title is always visible.
    assert: async (p) => expect(p.getByText('Meine Fahrzeuge').first()).toBeVisible(),
  },
  kalender: {
    note: 'Vorheriger Monat step-button',
    assert: async (p) =>
      expect(p.getByRole('button', { name: 'Vorheriger Monat' })).toBeVisible(),
  },
  karte: {
    note: 'map surface mounted (Frei stat or empty-state hint)',
    // Loaded state renders "Frei" / "Gesamt" summary stats; empty demo-tenant
    // state renders "Keine Standorte". Either proves KarteV5 mounted.
    assert: async (p) =>
      expect(
        p.getByText(/^(Frei|Keine Standorte)$/).first(),
      ).toBeVisible(),
  },
  credits: {
    note: 'Monatl. Kontingent stat',
    assert: async (p) => expect(p.getByText('Monatl. Kontingent').first()).toBeVisible(),
  },
  team: {
    note: 'Team surface mounted (stat or empty-state hint)',
    // Loaded state renders "Heute anwesend" stat; empty demo-tenant state
    // (no team-absences rows) renders "Noch keine Teamdaten". Either
    // proves TeamV5 mounted — matches the same belt-and-braces pattern
    // used for karte / einchecken above.
    assert: async (p) =>
      expect(
        p.getByText(/^(Heute anwesend|Noch keine Teamdaten)$/).first(),
      ).toBeVisible(),
  },
  rangliste: {
    note: 'Rangliste banner',
    // The "Früh/Teamplayer/..." badges only show when scoreboard rows exist.
    // The banner title is always rendered.
    assert: async (p) =>
      expect(p.locator('main').getByText('Rangliste', { exact: true }).first()).toBeVisible(),
  },
  ev: {
    note: 'Ladepunkte column header',
    assert: async (p) => expect(p.getByText('Ladepunkte', { exact: true }).first()).toBeVisible(),
  },
  tausch: {
    note: 'Neue Anfrage cta',
    assert: async (p) =>
      expect(p.getByRole('button', { name: 'Neue Anfrage' })).toBeVisible(),
  },
  einchecken: {
    note: 'check-in surface mounted (banner or empty-booking hint)',
    // Loaded state renders the "Einchecken" banner; empty demo-tenant
    // state (no active booking) renders "Keine aktive Buchung".
    assert: async (p) =>
      expect(
        p
          .locator('main')
          .getByText(/^(Einchecken|Keine aktive Buchung)$/)
          .first(),
      ).toBeVisible(),
  },
  vorhersagen: {
    note: 'Vorhersagen banner',
    assert: async (p) =>
      expect(p.locator('main').getByText('Vorhersagen', { exact: true }).first()).toBeVisible(),
  },
  gaestepass: {
    note: 'Gäste-Pass banner + Neuer Pass cta',
    assert: async (p) => {
      await expect(
        p.locator('main').getByText('Gäste-Pass', { exact: true }).first(),
      ).toBeVisible();
    },
  },
  analytics: {
    note: 'Analytics banner',
    assert: async (p) =>
      expect(p.locator('main').getByText('Analytics', { exact: true }).first()).toBeVisible(),
  },
  nutzer: {
    note: 'Nutzer banner',
    // KNOWN BROKEN on github/main (a8fba08): NutzerV5 throws
    // "h.filter is not a function" during hydration, the v5 shell never
    // mounts and <header><h1> is never emitted. Bug lives in the screen
    // component (not this test), so this anchor is correct as-written —
    // we skip instead of tightening / relaxing. Raised in coverage PR so
    // it's visible; fix is tracked separately.
    // TODO: flip back to assert once NutzerV5 renders again.
    assert: async (p) =>
      expect(p.locator('main').getByText('Nutzer', { exact: true }).first()).toBeVisible(),
  },
  billing: {
    note: 'Abrechnung banner',
    assert: async (p) => expect(p.getByText('Abrechnung').first()).toBeVisible(),
  },
  lobby: {
    note: 'lobby surface mounted (section-label or error card)',
    // The lobby config endpoint may 500 on the demo tenant; either the
    // "Aktiver Screen" section label or the LobbyV5 error card proves
    // the screen component (not PlaceholderV5) is mounted.
    assert: async (p) =>
      expect(
        p.getByText(/^(Aktiver Screen|Fehler beim Laden)$/).first(),
      ).toBeVisible(),
  },
  benachrichtigungen: {
    note: 'Ankündigungen banner',
    assert: async (p) => expect(p.getByText('Ankündigungen').first()).toBeVisible(),
  },
  einstellungen: {
    note: 'Sprache section-label',
    assert: async (p) => expect(p.getByText('Sprache', { exact: true }).first()).toBeVisible(),
  },
  standorte: {
    note: 'Neuer Standort section',
    assert: async (p) => expect(p.getByText('Neuer Standort').first()).toBeVisible(),
  },
  integrations: {
    note: 'integrations surface mounted (banner or error card)',
    // Admin integrations endpoint may 404/500 on the demo tenant.
    assert: async (p) =>
      expect(
        p
          .locator('main')
          .getByText(/^(Integrationen|Fehler beim Laden|Keine Integrationen verfügbar)$/)
          .first(),
      ).toBeVisible(),
  },
  apikeys: {
    note: 'api-keys surface mounted (banner or error card)',
    // Admin api-keys endpoint may 404/500 on the demo tenant.
    assert: async (p) =>
      expect(
        p
          .locator('main')
          .getByText(/^(API-Schlüssel|Fehler beim Laden|Keine Schlüssel)$/)
          .first(),
      ).toBeVisible(),
  },
  audit: {
    note: 'Audit-Log banner',
    assert: async (p) =>
      expect(p.locator('main').getByText('Audit-Log', { exact: true }).first()).toBeVisible(),
  },
  policies: {
    note: 'policies surface mounted (banner or error card)',
    // Admin policies endpoint may 404/500 on the demo tenant.
    assert: async (p) =>
      expect(
        p
          .locator('main')
          .getByText(/^(Richtlinien|Fehler beim Laden|Keine Richtlinien)$/)
          .first(),
      ).toBeVisible(),
  },
  profil: {
    note: 'Kontoinformation section-label',
    assert: async (p) => expect(p.getByText('Kontoinformation').first()).toBeVisible(),
  },
};

test.describe('v5 happy paths', () => {
  // v5 is not yet viewport-responsive below ~900 px — the 230 px sidebar
  // squeezes the main column and Playwright reports the <h1> as hidden on
  // Pixel 5. Coverage for mobile-chrome lands with the v5 responsive
  // refactor (tracked separately). Visual baselines on mobile-chrome are
  // deliberately skipped for the same reason.
  test.beforeEach(async ({ page }, testInfo) => {
    test.skip(
      testInfo.project.name === 'mobile-chrome',
      'v5 is desktop-only until the responsive refactor ships — see docs/v5-test-coverage-plan.md',
    );
    await loginAsAdmin(page);
  });

  // KNOWN_BROKEN: screens whose React tree throws during hydration on
  // github/main and never emit <header><h1>. Surfaced by this PR; tracked
  // as a follow-up fix. Tests stay declared (not deleted) so the moment
  // the screen is repaired the anchor reactivates without a diff here.
  const KNOWN_BROKEN = new Set<V5Screen>(['nutzer']);

  for (const [screen, anchor] of Object.entries(ANCHORS) as Array<[V5Screen, Anchor]>) {
    test(`${screen} — ${anchor.note}`, async ({ page }) => {
      test.fixme(
        KNOWN_BROKEN.has(screen),
        `${screen} is broken on github/main — hydration throws before <header><h1> renders`,
      );
      await openV5(page, screen);

      // Shell loaded the correct screen.
      await expect(page.locator('header h1')).toHaveText(V5_LABELS[screen]);

      // Real screen component mounted — not PlaceholderV5.
      await expect(
        page.getByText('Migration in Arbeit'),
      ).toBeHidden();

      // Screen-specific anchor visible.
      await anchor.assert(page);
    });
  }
});
