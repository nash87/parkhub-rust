import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { loginAsAdmin, openV5, V5_SCREENS } from './v5-helpers';

/**
 * v5 accessibility audit — T-1974 (Rust mirror of parkhub-php #346).
 *
 * One axe-core run per v5 screen (marble_light mode). Asserts zero
 * `serious` + `critical` WCAG 2.1 AA violations — same contract as the
 * existing root `e2e/a11y.spec.ts`, scoped to the v5 surface.
 *
 * `color-contrast` is disabled: v5 drives colour tokens via CSS custom
 * properties + OKLCH gradients (marble / void palettes are hand-tuned
 * in the design phase and re-audited via the design-system tooling).
 *
 * Runs under the `chromium` project only; the mobile-chrome breakpoint
 * lands with the v5 responsive refactor (see v5-visual.spec.ts header).
 */

test.describe('v5 a11y — WCAG 2.1 AA', () => {
  test.beforeEach(async ({ page }, testInfo) => {
    test.skip(
      testInfo.project.name !== 'chromium',
      'v5 a11y audit pinned to chromium — mobile variants land with the responsive refactor',
    );
    await loginAsAdmin(page);
  });

  for (const screen of V5_SCREENS) {
    test(`@a11y ${screen}: no serious or critical axe violations`, async ({ page }) => {
      await openV5(page, screen);

      const results = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22a', 'wcag22aa'])
        .disableRules(['color-contrast'])
        .analyze();

      const blockers = results.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical',
      );
      expect(
        blockers,
        JSON.stringify(blockers, null, 2),
      ).toEqual([]);
    });
  }
});
