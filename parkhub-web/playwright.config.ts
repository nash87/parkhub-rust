import { defineConfig } from '@playwright/test';

// Three run modes:
//
//   1. Default (CI/cloud)       baseURL = https://parkhub-rust-demo.onrender.com
//   2. CI-local                 E2E_BASE_URL=http://localhost:8081 (server already up)
//   3. Dev-local (hermetic)     E2E_LOCAL=1 — Playwright spins up `npm run dev` on :4321
//                               and proxies /api to a pre-running parkhub-server on :8081
//
// For full AI-driven vibe-coding against a running stack, mode 3 is the move.
// See scripts/e2e-local.sh for the one-command bootstrap.
const LOCAL_DEV = process.env.E2E_LOCAL === '1';
const BASE = process.env.E2E_BASE_URL
  || process.env.BASE_URL
  || (LOCAL_DEV ? 'http://localhost:4321' : 'https://parkhub-rust-demo.onrender.com');

export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: { timeout: 30_000 },
  retries: process.env.CI ? 2 : 0,
  workers: 1, // Sequential — Render free tier can't handle parallel
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? [['github'], ['list']] : [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: BASE,
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'on-first-retry',
    video: 'retain-on-failure',
    actionTimeout: 15_000,
    navigationTimeout: 45_000,
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
  ],
  // Only start the Astro dev server when running in hermetic-local mode.
  // CI and cloud runs assume an already-running target.
  webServer: LOCAL_DEV ? {
    command: 'npm run dev',
    url: 'http://localhost:4321',
    reuseExistingServer: true,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  } : undefined,
});
