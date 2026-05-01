import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { gotoAppPage } from './helpers';

const routes = ['/', '/login', '/register', '/setup'];

for (const route of routes) {
  test(`a11y: ${route} has no critical violations`, async ({ page }) => {
    await gotoAppPage(page, route);
    const results = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'])
      .disableRules(['color-contrast']) // managed via CSS custom properties
      .analyze();

    const serious = results.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical'
    );
    expect(serious).toEqual([]);
  });
}
