import { test, expect } from './fixtures/axe';

// claude.ai/design integration (#335) — broad axe-core pass across the key
// post-login surfaces to catch a11y regressions introduced by design churn.
// Individual dialogs are covered by design-shortcuts-help + design-assistant;
// this file audits the persistent page chrome.

async function login(page: any) {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('parkhub_welcome_seen', '1'));
  await page.goto('/login');
  await page.waitForSelector('#demo-autofill', { timeout: 45_000 });
  await page.click('#demo-autofill');
  await page.click('#login-submit');
  await page.waitForSelector('text=Active Bookings', { timeout: 30_000 });
}

const SURFACES: Array<[string, string]> = [
  ['/', 'Active Bookings'],
  ['/bookings', 'booking'],
  ['/profile', 'profile'],
  ['/settings', 'appearance'],
];

test.describe('Design — axe coverage across post-login surfaces', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  for (const [path, anchor] of SURFACES) {
    test(`${path} passes axe WCAG 2.1 AA`, async ({ page, axe }) => {
      await page.goto(path);
      await page.getByText(new RegExp(anchor, 'i')).first().waitFor({ timeout: 15_000 });
      await axe({ exclude: ['.leaflet-container', '[data-testid="map-canvas"]'] });
    });
  }
});
