import { defineConfig, devices } from '@playwright/test';

// Local E2E runs against one local server process plus a frontend dev server.
// Defaulting to one worker avoids false-red loading flakes; override explicitly
// with PLAYWRIGHT_WORKERS when the local stack can sustain more concurrency.
const localWorkers = process.env.PLAYWRIGHT_WORKERS
  ? Number(process.env.PLAYWRIGHT_WORKERS)
  : 1;
const includeWebkitProject = !!process.env.CI || process.env.PLAYWRIGHT_ENABLE_WEBKIT === '1';

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : localWorkers,
  reporter: [
    ['html', { open: 'never' }],
    ['list'],
  ],
  use: {
    baseURL: process.env.E2E_BASE_URL || 'http://localhost:8081',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'mobile-chrome', use: { ...devices['Pixel 5'] } },
    ...(includeWebkitProject
      ? [{ name: 'mobile-safari', use: { ...devices['iPhone 14'] } }]
      : []),
  ],
});
