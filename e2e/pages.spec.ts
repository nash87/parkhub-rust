import { existsSync, readFileSync } from 'node:fs';
import { test, expect, type Page } from '@playwright/test';
import { gotoAppPage, loginBrowserViaApi, PUBLIC_ROUTES, PROTECTED_ROUTES, ADMIN_ROUTES } from './helpers';

const UNPROFESSIONAL_ROUTE_COPY = /\bAI-powered\b|\bAI-generated\b|\bKI-powered\b|lorem ipsum|Migration in Arbeit|Generative Background|Generativer Hintergrund|generative art|Fondo generativo|Sfondo generativo|Fundo generativo|Arrière-plan génératif|Generatywne tło|Üretken Arka Plan|生成式背景|生成背景|MARMOR GOVERNANCE STUDIO|OPERATIVER FOKUS|Void finance desk|Marble finance desk|Finance pulse|Void analytics deck|Marble analytics deck|Void user ledger|Marble user ledger|Control posture|Void fleet deck|Marble vehicle registry|Void favorites deck|Marble saved slots|Void charging deck|Marble charging suite|Void fleet monitor|Marble ops overview|Marble Surface|Void Surface|Void balance deck|Marble credit ledger|Void schedule room|Marble calendar deck|Void signal board|Marble inbox board|Void guest desk|Marble guest desk|Void occupancy deck|Marble live map board|Void pass wallet|Marble pass desk|Void check-in flow|Marble arrival lane|Void history ledger|Marble history ledger|Void identity suite|Marble profile deck|Void signal|Marble editorial|Marble preference deck|Void exchange board|Marble exchange board|Void team deck|Marble roster board/i;

function readAppRoutesSource() {
  return readFileSync('parkhub-web/src/App.tsx', 'utf8');
}

function uniqueRoutes(routes: string[]) {
  return new Set(routes).size;
}

type RouteGroups = {
  publicRoutes: string[];
  protectedRoutes: string[];
  adminRoutes: string[];
};

function smokeRoute(routePath: string) {
  return routePath.replace(':lotId', '1');
}

function extractRouteGroups(appSource: string): RouteGroups {
  const layoutStart = appSource.indexOf('<Route path="/"');
  const adminStart = appSource.indexOf('<Route path="admin"');
  const wildcardStart = appSource.indexOf('<Route path="*"');
  expect(layoutStart).toBeGreaterThan(0);
  expect(adminStart).toBeGreaterThan(layoutStart);
  expect(wildcardStart).toBeGreaterThan(adminStart);

  const publicBlock = appSource.slice(appSource.indexOf('<Routes'), layoutStart);
  const protectedBlock = appSource.slice(layoutStart, adminStart);
  const adminBlock = appSource.slice(adminStart, wildcardStart);

  const publicRoutes = [...publicBlock.matchAll(/<Route path="(\/[^"]+)"/g)]
    .map(match => smokeRoute(match[1]));

  const protectedRoutes = [...protectedBlock.matchAll(/<Route (index\b|path="([^"]+)")/g)]
    .map((match) => {
      if (match[1] === 'index') return '/';
      const path = match[2];
      return path === '/' ? null : `/${path}`;
    })
    .filter((route): route is string => Boolean(route));

  const adminRoutes = [
    '/admin',
    ...[...adminBlock.matchAll(/<Route path="([^"]+)"/g)]
      .map((match) => match[1])
      .filter(path => path !== 'admin')
      .map(path => `/admin/${path}`),
  ];

  return { publicRoutes, protectedRoutes, adminRoutes };
}

function lazyModuleTestPath(modulePath: string) {
  return `parkhub-web/src/${modulePath.slice('./'.length)}.test.tsx`;
}

test('route smoke lists stay in lockstep with App.tsx', () => {
  const appSource = readAppRoutesSource();
  const routeGroups = extractRouteGroups(appSource);

  expect(PUBLIC_ROUTES).toEqual(routeGroups.publicRoutes);
  expect(PROTECTED_ROUTES).toEqual(routeGroups.protectedRoutes);
  expect(ADMIN_ROUTES).toEqual(routeGroups.adminRoutes);
  expect(uniqueRoutes(PUBLIC_ROUTES)).toBe(PUBLIC_ROUTES.length);
  expect(uniqueRoutes(PROTECTED_ROUTES)).toBe(PROTECTED_ROUTES.length);
  expect(uniqueRoutes(ADMIN_ROUTES)).toBe(ADMIN_ROUTES.length);
});

test('routed lazy modules keep colocated component coverage', () => {
  const appSource = readAppRoutesSource();
  const lazyModuleTestPaths = [...appSource.matchAll(/lazy\(\(\) => import\('([^']+)'\),/g)]
    .map(match => lazyModuleTestPath(match[1]));
  const missingTests = lazyModuleTestPaths.filter(path => !existsSync(path));

  expect(missingTests).toEqual([]);
});

async function expectKnownRouteShell(page: Page) {
  const body = page.locator('body');
  await expect(body).not.toContainText(/\b404\b|not found|nicht gefunden/i);
  await expect(body).not.toContainText(UNPROFESSIONAL_ROUTE_COPY);
  await expect(page.locator('.blur-3xl')).toHaveCount(0);
}

test.describe('Pages — Public Routes', () => {
  for (const route of PUBLIC_ROUTES) {
    test(`${route} loads without errors`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      // Page should render some content
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
    });
  }

  test('/login shows a login form', async ({ page }) => {
    await gotoAppPage(page, '/login');
    await expect(page.getByRole('button', { name: /sign in|log in|login/i })).toBeVisible();
  });

  test('/register shows a registration form', async ({ page }) => {
    await gotoAppPage(page, '/register');
    await expect(page.getByRole('button', { name: /sign up|register|create/i })).toBeVisible();
  });

  test('/forgot-password shows password reset form', async ({ page }) => {
    await gotoAppPage(page, '/forgot-password');
    await expect(page.getByLabel(/email/i)).toBeVisible();
  });
});

test.describe('Pages — Protected Routes (after login)', () => {
  test.beforeEach(async ({ page }) => {
    await loginBrowserViaApi(page);
  });

  for (const route of PROTECTED_ROUTES) {
    test(`${route} loads after auth`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
    });
  }

  test('dashboard shows stats or content', async ({ page }) => {
    await gotoAppPage(page, '/');
    // Dashboard should have some visible element
    await expect(page.locator('main, [data-testid], h1, h2')).not.toHaveCount(0);
  });

  test('/profile page has settings section', async ({ page }) => {
    await gotoAppPage(page, '/profile');
    await expect(page.locator('body')).toContainText(/profile|settings|theme|account/i);
  });
});

test.describe('Pages — Admin Routes (after admin login)', () => {
  test.beforeEach(async ({ page }) => {
    await loginBrowserViaApi(page);
  });

  for (const route of ADMIN_ROUTES) {
    test(`${route} loads for admin`, async ({ page }) => {
      const res = await gotoAppPage(page, route);
      expect(res?.status()).toBeLessThan(400);
      await expect(page.locator('body')).not.toBeEmpty();
      await expectKnownRouteShell(page);
      await expect(page.locator('body')).not.toContainText(/returned an object instead of string/i);
    });
  }

  test('admin modules, chargers, and updates settle without broken UI chrome', async ({ page }) => {
    for (const route of ['/admin/modules', '/admin/chargers', '/admin/updates']) {
      await gotoAppPage(page, route);
      await expect(page.locator('body')).not.toContainText(/returned an object instead of string/i);
      await expect(page.locator('body')).not.toContainText(/key '[^']+' returned/i);
      await expect(page.locator('h1:visible, h2:visible').first()).toBeVisible();
    }

    await gotoAppPage(page, '/admin/chargers');
    await expect(page.getByText(/Total Chargers/i).first()).toBeVisible();
    await expect(page.locator('body')).not.toContainText(/Could not load charger statistics/i);

    await gotoAppPage(page, '/admin/updates');
    await expect(page.getByTestId('current-version')).not.toHaveText('—');
  });
});

test.describe('Pages — Redirects', () => {
  test('/ without auth redirects to /login or /welcome', async ({ page }) => {
    await gotoAppPage(page, '/');
    // AuthProvider shows LoadingSplash on mount until /api/v1/users/me
    // resolves, so URL rewrites to /login or /welcome only AFTER the
    // unauth check returns. Wait for the redirect before reading URL.
    await page.waitForURL(/\/(login|welcome)/, { timeout: 10_000, waitUntil: 'commit' });
  });

  test('unknown route shows 404 page', async ({ page }) => {
    await gotoAppPage(page, '/this-route-does-not-exist');
    await expect(page.locator('body')).toContainText(/not found|404/i);
  });
});
