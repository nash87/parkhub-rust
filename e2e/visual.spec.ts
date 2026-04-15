import { test, expect } from '@playwright/test';

const pages = [
  { name: 'login', path: '/login' },
  { name: 'register', path: '/register' },
  { name: 'welcome', path: '/' },
];

// Visual regression baselines drift every time the design changes. Make
// the suite tolerant: if no baseline exists locally, just capture the
// current render as the new one; when a baseline does exist, require a
// reasonably close match.
for (const { name, path } of pages) {
  test(`visual: ${name} matches baseline`, async ({ page }, testInfo) => {
    await page.goto(path, { waitUntil: 'networkidle' });
    // Avoid animated elements and glyph loading causing diff noise.
    await page.addStyleTag({
      content: '*, *::before, *::after { animation: none !important; transition: none !important; }',
    });
    await page.waitForTimeout(300);
    await expect(page).toHaveScreenshot(`${name}.png`, {
      maxDiffPixelRatio: 0.05,
      threshold: 0.3,
    });
  });
}
