import { type Page, type APIRequestContext, type Response } from '@playwright/test';

type AppNavigationOptions = Omit<NonNullable<Parameters<Page['goto']>[1]>, 'waitUntil'>;

const APP_NAVIGATION_TIMEOUT_MS = 8_000;
const APP_SHELL_SETTLE_TIMEOUT_MS = 2_500;
const TRANSIENT_MODULE_IMPORT_ERROR =
  /Importing a module script failed|Failed to fetch dynamically imported module/i;

/** Demo credentials used across all E2E tests. */
export const DEMO_ADMIN = {
  username: 'admin',
  email: 'admin@parkhub.test',
  password: 'demo',
};

/** Authenticate via API and return the JWT token. */
export async function loginViaApi(request: APIRequestContext): Promise<string> {
  const res = await request.post('/api/v1/auth/login', {
    data: { username: DEMO_ADMIN.username, password: DEMO_ADMIN.password },
  });
  const body = await res.json();
  return body.data?.tokens?.access_token ?? body.data?.token ?? body.token ?? '';
}

/**
 * Authenticate the browser context without driving the UI login redirect.
 *
 * Protected route smoke tests only need an authenticated browser. Using the
 * context-scoped API client keeps the httpOnly cookie in the same jar as the
 * page and avoids racing a second hard navigation against the post-login SPA
 * lazy imports on WebKit mobile.
 */
export async function loginBrowserViaApi(page: Page): Promise<string> {
  const res = await page.context().request.post('/api/v1/auth/login', {
    data: { username: DEMO_ADMIN.username, password: DEMO_ADMIN.password },
  });
  const body = await res.json().catch(() => ({}));
  const token = body.data?.tokens?.access_token ?? body.data?.token ?? body.token ?? '';
  if (!res.ok() || !token) {
    throw new Error(`API login failed with HTTP ${res.status()}`);
  }

  // The Rust API correctly marks the real auth cookie Secure. Local E2E runs
  // against http://*.test, so force an HTTP-compatible cookie for hard reloads.
  await page.context().addCookies([{
    name: 'parkhub_token',
    value: token,
    url: new URL(res.url()).origin,
    httpOnly: true,
    secure: false,
    sameSite: 'Lax',
  }]);

  return token;
}

/**
 * Navigate inside the app using the readiness signal that matches this SPA.
 *
 * Playwright's default page.goto() waits for the window "load" event. WebKit
 * mobile emulation can intermittently miss load-state signals on the React/PWA
 * shell, which turns healthy pages into 30s navigation timeouts. Waiting for
 * commit keeps the navigation itself bounded; the suite then asserts concrete
 * page content after navigation.
 */
export async function gotoAppPage(
  page: Page,
  url: string,
  options: AppNavigationOptions = {},
): Promise<Response | null> {
  const timeout = options.timeout ?? APP_NAVIGATION_TIMEOUT_MS;
  let lastError: unknown;

  for (let attempt = 0; attempt < 2; attempt += 1) {
    try {
      const response = await page.goto(url, { ...options, timeout, waitUntil: 'commit' });
      await waitForAppDomReady(page);
      if (!(await waitForAppShellSettled(page))) {
        await reloadAppPage(page);
        await waitForAppShellSettled(page);
      }
      await recoverFromTransientModuleImportError(page, url);
      return response;
    } catch (error) {
      lastError = error;
      if (attempt > 0 || !isNavigationTimeout(error)) {
        throw error;
      }

      await page.evaluate(() => window.stop()).catch(() => undefined);
      await page.waitForTimeout(250);
    }
  }

  throw lastError instanceof Error ? lastError : new Error(String(lastError));
}

export async function waitForAppDomReady(page: Page): Promise<void> {
  await page.waitForLoadState('domcontentloaded', { timeout: 10_000 }).catch(() => undefined);
}

async function recoverFromTransientModuleImportError(page: Page, url: string): Promise<void> {
  if (!(await hasTransientModuleImportError(page))) {
    return;
  }

  await page.evaluate(() => window.stop()).catch(() => undefined);
  await page.waitForTimeout(500);
  await reloadAppPage(page);
  await waitForAppShellSettled(page);

  if (!(await hasTransientModuleImportError(page))) {
    return;
  }

  await page.evaluate(() => window.stop()).catch(() => undefined);
  await page.waitForTimeout(500);
  await page.goto(url, { waitUntil: 'commit', timeout: APP_NAVIGATION_TIMEOUT_MS });
  await waitForAppDomReady(page);
  await waitForAppShellSettled(page);

  if (await hasTransientModuleImportError(page)) {
    throw new Error(`App module import failed after recovery while navigating to ${url}`);
  }
}

async function reloadAppPage(page: Page): Promise<void> {
  await page.reload({ waitUntil: 'commit', timeout: APP_NAVIGATION_TIMEOUT_MS });
  await waitForAppDomReady(page);
}

async function waitForAppShellSettled(page: Page): Promise<boolean> {
  return page.waitForFunction(
    () => {
      const text = document.body?.innerText?.trim() ?? '';
      return text.length >= 4
        || text.includes('Importing a module script failed')
        || text.includes('Failed to fetch dynamically imported module');
    },
    undefined,
    { timeout: APP_SHELL_SETTLE_TIMEOUT_MS },
  ).then(() => true, () => false);
}

async function hasTransientModuleImportError(page: Page): Promise<boolean> {
  const text = await page.locator('body').textContent({ timeout: 500 }).catch(() => '');
  return TRANSIENT_MODULE_IMPORT_ERROR.test(text ?? '');
}

function isNavigationTimeout(error: unknown): boolean {
  return error instanceof Error && /Timeout.*exceeded|page\.goto: Timeout/i.test(error.message);
}

/** Log in through the UI login form. */
export async function loginViaUi(page: Page): Promise<void> {
  await gotoAppPage(page, '/login');
  // Try username field first (Rust), then email field (PHP)
  const usernameField = page.locator('input[name="username"], input[type="email"], input[name="email"]').first();
  await usernameField.fill(DEMO_ADMIN.username);
  await page.locator('input[type="password"], input[name="password"]').first().fill(DEMO_ADMIN.password);
  await page.getByRole('button', { name: /sign in|log in|login/i }).click();
  // Wait for redirect away from login page
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 10_000, waitUntil: 'commit' });
  // WebKit / mobile-safari commits Set-Cookie noticeably later than Chromium,
  // which races with the caller's subsequent page.goto('/protected-route'):
  // the second navigation runs before the HttpOnly auth cookie lands, the
  // server redirects to /login, and the test times out. Poll the browser
  // context until the server-issued cookie is actually present.
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    const cookies = await page.context().cookies();
    if (cookies.some((c) => c.name === 'parkhub_token' || c.name === 'laravel_session' || c.name === 'XSRF-TOKEN')) {
      return;
    }
    await page.waitForTimeout(100);
  }
}

/** All public frontend routes (no auth needed). Keep in lockstep with App.tsx. */
export const PUBLIC_ROUTES = ['/welcome', '/tour', '/login', '/register', '/forgot-password', '/choose', '/lobby/1', '/setup'];

/** All protected frontend routes (auth needed). */
export const PROTECTED_ROUTES = [
  '/',
  '/book',
  '/bookings',
  '/credits',
  '/vehicles',
  '/favorites',
  '/absences',
  '/profile',
  '/team',
  '/notifications',
  '/calendar',
  '/visitors',
  '/ev-charging',
  '/history',
  '/absence-approval',
  '/map',
  '/swap-requests',
  '/checkin',
  '/guest-pass',
  '/leaderboard',
  '/predict',
  '/translations',
  '/settings',
];

/** Admin-only frontend routes. */
export const ADMIN_ROUTES = [
  '/admin',
  '/admin/settings',
  '/admin/users',
  '/admin/lots',
  '/admin/announcements',
  '/admin/reports',
  '/admin/translations',
  '/admin/analytics',
  '/admin/rate-limits',
  '/admin/tenants',
  '/admin/modules',
  '/admin/audit-log',
  '/admin/data',
  '/admin/fleet',
  '/admin/accessible',
  '/admin/maintenance',
  '/admin/billing',
  '/admin/visitors',
  '/admin/chargers',
  '/admin/widgets',
  '/admin/plugins',
  '/admin/compliance',
  '/admin/sso',
  '/admin/webhooks',
  '/admin/roles',
  '/admin/zones',
  '/admin/updates',
  '/admin/heatmap',
  '/admin/scheduled-reports',
];

/** All public API endpoints that should return 200 without auth. */
export const PUBLIC_API_ENDPOINTS = [
  '/health',
  '/health/live',
  '/health/ready',
  '/api/v1/modules',
  '/api/v1/system/version',
  '/api/v1/system/maintenance',
  '/api/v1/public/occupancy',
  '/api/v1/setup/status',
];

/** Protected API endpoints that require auth (GET). */
export const PROTECTED_API_ENDPOINTS = [
  '/api/v1/me',
  '/api/v1/users/me',
  '/api/v1/lots',
  '/api/v1/bookings',
  '/api/v1/user/stats',
  '/api/v1/user/preferences',
  '/api/v1/team',
  '/api/v1/team/today',
  '/api/v1/notifications',
];

/** Admin API endpoints (GET). */
export const ADMIN_API_ENDPOINTS = [
  '/api/v1/admin/users',
  '/api/v1/admin/bookings',
  '/api/v1/admin/stats',
  '/api/v1/admin/settings',
  '/api/v1/admin/audit-log',
];

/** Mobile device viewports for responsive tests. */
export const MOBILE_DEVICES = [
  { name: 'iPhone 14 Pro', width: 393, height: 852 },
  { name: 'iPhone 15 Pro Max', width: 430, height: 932 },
  { name: 'Samsung Galaxy S24', width: 360, height: 780 },
  { name: 'iPad Pro', width: 1024, height: 1366 },
  { name: 'Pixel 8', width: 412, height: 915 },
];
