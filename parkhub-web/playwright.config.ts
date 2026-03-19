import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 90_000,
  expect: { timeout: 30_000 },
  retries: 2,
  workers: 1, // Sequential — Render free tier can't handle parallel
  use: {
    baseURL: process.env.BASE_URL || 'https://parkhub-rust-demo.onrender.com',
    headless: true,
    screenshot: 'only-on-failure',
    trace: 'on-first-retry',
    actionTimeout: 15_000,
    navigationTimeout: 45_000,
  },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
  ],
});
