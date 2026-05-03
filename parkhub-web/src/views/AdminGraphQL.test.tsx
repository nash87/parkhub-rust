import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'admin.overview': 'Overview',
        'admin.settings': 'Settings',
        'admin.users': 'Users',
        'admin.lots': 'Lots',
        'admin.announcements': 'Announcements',
        'admin.reports': 'Reports',
        'admin.translations': 'Translations',
        'admin.rateLimits': 'Rate Limits',
        'admin.tenants': 'Tenants',
        'admin.auditLog': 'Audit Log',
        'admin.dataManagement': 'Data',
        'admin.fleet': 'Fleet',
        'admin.accessible': 'Accessible',
        'admin.maintenance': 'Maintenance',
        'admin.billing': 'Billing',
        'admin.visitors': 'Visitors',
        'admin.chargers': 'EV Chargers',
        'admin.plugins': 'Plugins',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, custom, layoutId, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    aside: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, custom, layoutId, ...props }: any, ref: any) => (
      <aside ref={ref} {...props}>{children}</aside>
    )),
    span: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, custom, layoutId, ...props }: any, ref: any) => (
      <span ref={ref} {...props}>{children}</span>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@phosphor-icons/react')>();
  const icon = (props: any) => <span {...props} />;
  return {
    ...actual,
    ChartBarIcon: icon,
    GearSixIcon: icon,
    UsersIcon: icon,
    MegaphoneIcon: icon,
    ChartLineIcon: icon,
    MapPinIcon: icon,
    TranslateIcon: icon,
    PresentationChartIcon: icon,
    GaugeIcon: icon,
    BuildingsIcon: icon,
    ClockCounterClockwiseIcon: icon,
    DatabaseIcon: icon,
    CarIcon: icon,
    WheelchairIcon: icon,
    WrenchIcon: icon,
    CurrencyDollarIcon: icon,
    UserPlusIcon: icon,
    LightningIcon: icon,
    PuzzlePieceIcon: icon,
    GraphicsCardIcon: icon,
    ShieldCheckIcon: icon,
    LockKeyIcon: icon,
    MapTrifoldIcon: icon,
    ClockIcon: icon,
    WebhooksLogoIcon: icon,
    ArrowsClockwiseIcon: icon,
    ListIcon: icon,
    XIcon: icon,
    ArrowSquareOutIcon: icon,
  };
});

import { AdminPage } from './Admin';

describe('Admin GraphQL navigation', () => {
  it('renders GraphQL playground link in admin nav', () => {
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <AdminPage />
      </MemoryRouter>
    );
    // New sidebar uses the longer "GraphQL Playground" label; accept either
    // in case the label shortens again in future.
    expect(screen.getByText(/GraphQL/)).toBeInTheDocument();
  });

  it('renders Plugins link in admin nav', () => {
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <AdminPage />
      </MemoryRouter>
    );
    expect(screen.getAllByText('Plugins').length).toBeGreaterThan(0);
  });

  it('GraphQL link points to playground endpoint', () => {
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <AdminPage />
      </MemoryRouter>
    );
    const link = screen.getByText(/GraphQL/).closest('a');
    expect(link).toBeTruthy();
    expect(link?.getAttribute('href')).toBe('/api/v1/graphql/playground');
  });
});
