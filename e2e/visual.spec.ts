import { test, expect } from '@playwright/test';

const pages = [
  { name: 'login', path: '/login' },
  { name: 'register', path: '/register' },
  { name: 'welcome', path: '/' },
];

// Visual snapshots are only maintained for the chromium project — mobile
// projects don't have committed baselines, so running the snapshot step
// on those projects would just add CI noise without catching regressions.
for (const { name, path } of pages) {
  test(`visual: ${name} matches baseline`, async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== 'chromium', 'visual baselines are chromium-only');

    await page.goto(path);
    await page.waitForLoadState('domcontentloaded');
    await expect(page).toHaveScreenshot(`${name}.png`, {
      maxDiffPixelRatio: 0.01,
      threshold: 0.2,
    });
  });
}
