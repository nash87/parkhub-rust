// Capture README screenshots for parkhub-rust against a local parkhub-server.
// Run: BASE_URL=http://127.0.0.1:18181 node scripts/capture-screenshots.mjs
// Outputs to ../screenshots/ (relative to parkhub-web/ = parkhub-rust/screenshots).

import { chromium } from 'playwright';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';

const BASE = process.env.BASE_URL || 'http://127.0.0.1:18181';
const OUT = resolve(import.meta.dirname ?? '.', '..', '..', 'screenshots');
const USER = 'admin';
const PASS = 'demo';

const shots = [
  { file: '01-login.png',          path: '/login',          auth: false },
  { file: '02-dashboard.png',      path: '/',               auth: true },
  { file: '03-register.png',       path: '/register',       auth: false },
  { file: '05-book.png',           path: '/book',           auth: true },
  { file: '06-bookings.png',       path: '/bookings',       auth: true },
  { file: '07-vehicles.png',       path: '/vehicles',       auth: true },
  { file: '08-admin.png',          path: '/admin',          auth: true },
  { file: '09-dark-mode.png',      path: '/',               auth: true, dark: true },
  { file: '10-modules-dashboard.png', path: '/admin/modules',    auth: true, admin: true },
  { file: '11-command-palette.png',   path: '/',                 auth: true, palette: true },
];

await mkdir(OUT, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await context.newPage();

async function login() {
  await page.goto(`${BASE}/login`, { waitUntil: 'domcontentloaded' });
  const userField = page.locator('input[name="username"], input[type="email"], input[name="email"]').first();
  await userField.fill(USER);
  await page.locator('input[type="password"]').first().fill(PASS);
  await page.getByRole('button', { name: /sign in|log in|login/i }).click();
  await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 15_000 });
  // WebKit cookie race — poll for cookie landing
  for (let i = 0; i < 50; i++) {
    const cookies = await context.cookies();
    if (cookies.some((c) => c.name === 'laravel_session' || c.name === 'XSRF-TOKEN' || c.name === 'parkhub_token')) break;
    await page.waitForTimeout(100);
  }
}

async function captureOne(shot) {
  if (shot.auth) {
    const cookies = await context.cookies();
    const loggedIn = cookies.some((c) => c.name === 'laravel_session' || c.name === 'parkhub_token');
    if (!loggedIn) await login();
  }
  await page.goto(`${BASE}${shot.path}`, { waitUntil: 'domcontentloaded' });
  if (shot.dark) {
    await page.evaluate(() => {
      localStorage.setItem('parkhub_theme', 'dark');
      document.documentElement.classList.add('dark');
    });
    await page.waitForTimeout(300);
  }
  await page.waitForTimeout(1_500); // let lazy chunks + animations settle
  if (shot.palette) {
    // Open Cmd+K / Ctrl+K palette
    await page.keyboard.press('Control+K');
    await page.waitForTimeout(500);
  }
  const outPath = resolve(OUT, shot.file);
  await page.screenshot({ path: outPath, fullPage: false });
  console.log('✓', shot.file);
}

try {
  for (const s of shots) {
    try {
      await captureOne(s);
    } catch (e) {
      console.warn('✗', s.file, '—', e.message);
    }
  }
} finally {
  await browser.close();
}
