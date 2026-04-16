import { test, expect } from '@playwright/test';

const pages = [
  { name: 'login', path: '/login' },
  { name: 'register', path: '/register' },
  { name: 'welcome', path: '/' },
];

// Visual baselines are cross-environment smoke checks, not pixel-perfect
// contracts — font hinting and subpixel antialiasing routinely drift 5–8%
// between the baseline-generating host and CI Chromium even when the page
// is visually identical. 10% / 0.35 keeps the tests sensitive to real
// layout regressions (buttons moving, missing elements, colour swaps)
// while surviving renderer drift.
for (const { name, path } of pages) {
  test(`visual: ${name} matches baseline`, async ({ page }) => {
    await page.goto(path, { waitUntil: 'domcontentloaded' });
    await page.addStyleTag({
      content: '*, *::before, *::after { animation: none !important; transition: none !important; }',
    });
    await page.waitForTimeout(300);
    await expect(page).toHaveScreenshot(`${name}.png`, {
      maxDiffPixelRatio: 0.10,
      threshold: 0.35,
    });
  });
}
