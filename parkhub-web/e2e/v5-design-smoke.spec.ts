import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { loginAsAdmin, openV5, V5_LABELS, V5_SCREENS, type V5Screen } from './v5-helpers';

const REPRESENTATIVE_A11Y_SCREENS: V5Screen[] = ['dashboard', 'buchen', 'nutzer'];

function attachRuntimeGuard(page: Page): string[] {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) {
      errors.push(message.text());
    }
  });
  page.on('pageerror', (error) => {
    errors.push(error.message);
  });
  return errors;
}

test.describe('v5 design smoke', () => {
  test.beforeEach(async ({ page }) => {
    await loginAsAdmin(page);
  });

  for (const screen of V5_SCREENS) {
    test(`${screen} renders the real screen shell without runtime errors`, async ({ page }) => {
      const runtimeErrors = attachRuntimeGuard(page);

      await openV5(page, screen);

      await expect(page.locator('header h1')).toHaveText(V5_LABELS[screen]);
      await expect(page.getByText('Migration in Arbeit')).toBeHidden();
      await expect(page.locator('main')).toContainText(/\S/);
      expect(runtimeErrors, runtimeErrors.join('\n')).toEqual([]);
    });
  }

  test('keyboard focus enters the command surface', async ({ page }, testInfo) => {
    test.skip(
      testInfo.project.name !== 'chromium',
      'keyboard focus is the blocking desktop interaction gate',
    );

    await openV5(page, 'dashboard');
    await page.keyboard.press('Tab');

    await expect
      .poll(async () =>
        page.evaluate(() => {
          const active = document.activeElement;
          if (!active || active === document.body) return false;
          return active.matches('a,button,input,select,textarea,[tabindex]:not([tabindex="-1"])');
        }),
      )
      .toBe(true);
  });

  for (const screen of REPRESENTATIVE_A11Y_SCREENS) {
    test(`${screen} has no serious or critical axe violations`, async ({ page }, testInfo) => {
      test.skip(
        testInfo.project.name !== 'chromium',
        'axe gate runs on the desktop accessibility baseline',
      );

      await openV5(page, screen);

      const results = await new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22a', 'wcag22aa'])
        .disableRules(['color-contrast'])
        .analyze();

      const blockers = results.violations.filter(
        (violation) => violation.impact === 'serious' || violation.impact === 'critical',
      );
      expect(blockers, JSON.stringify(blockers, null, 2)).toEqual([]);
    });
  }
});
