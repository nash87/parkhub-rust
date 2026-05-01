import { type Page, type APIRequestContext, type Response } from '@playwright/test';

type AppNavigationOptions = Omit<NonNullable<Parameters<Page['goto']>[1]>, 'waitUntil'>;

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
 * Navigate inside the app using the readiness signal that matches this SPA.
 *
 * Playwright's default page.goto() waits for the window "load" event. WebKit
 * mobile emulation can intermittently miss that signal on the React/PWA shell
 * while the DOM is already ready and usable, which turns healthy pages into
 * 30s navigation timeouts. DOMContentLoaded is the stable boundary the E2E
 * suite already asserts after navigation.
 */
export async function gotoAppPage(
  page: Page,
  url: string,
  options: AppNavigationOptions = {},
): Promise<Response | null> {
  return page.goto(url, { ...options, waitUntil: 'domcontentloaded' });
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
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 10_000 });
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

/** All public frontend routes (no auth needed). */
export const PUBLIC_ROUTES = ['/login', '/register', '/forgot-password', '/welcome'];

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
  '/translations',
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
