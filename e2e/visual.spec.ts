import { test, expect } from '@playwright/test';

const pages = [
  { name: 'login', path: '/login' },
  { name: 'register', path: '/register' },
  { name: 'welcome', path: '/' },
];

for (const { name, path } of pages) {
  test(`visual: ${name} matches baseline`, async ({ page }) => {
    await page.goto(path);
    await page.waitForLoadState('networkidle');
    await expect(page).toHaveScreenshot(`${name}.png`, {
      maxDiffPixelRatio: 0.01,
      threshold: 0.2,
    });
  });
}
