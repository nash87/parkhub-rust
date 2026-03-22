import { type Page, type APIRequestContext } from '@playwright/test';

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

/** Log in through the UI login form. */
export async function loginViaUi(page: Page): Promise<void> {
  await page.goto('/login');
  // Try username field first (Rust), then email field (PHP)
  const usernameField = page.locator('input[name="username"], input[type="email"], input[name="email"]').first();
  await usernameField.fill(DEMO_ADMIN.username);
  await page.locator('input[type="password"], input[name="password"]').first().fill(DEMO_ADMIN.password);
  await page.getByRole('button', { name: /sign in|log in|login/i }).click();
  // Wait for redirect away from login page
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 10_000 });
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
  '/api/v1/features',
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
