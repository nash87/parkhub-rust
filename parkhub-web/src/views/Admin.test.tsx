import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen, within } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
  Outlet: () => <div data-testid="outlet">Outlet Content</div>,
  useLocation: () => ({ pathname: '/admin' }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'admin.title': 'Administration',
        'admin.subtitle': 'Manage your ParkHub instance',
        'admin.overview': 'Overview',
        'admin.settings': 'Settings',
        'admin.users': 'Users',
        'admin.lots': 'Parking Lots',
        'admin.announcements': 'Announcements',
        'admin.reports': 'Reports',
        'admin.translations': 'Translations',
        'admin.rateLimits': 'Rate Limits',
        'admin.tenants': 'Tenants',
      };
      return map[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    aside: React.forwardRef(({ children, ...props }: any, ref: any) => (
      <aside ref={ref} {...props}>{children}</aside>
    )),
    span: React.forwardRef(({ children, ...props }: any, ref: any) => (
      <span ref={ref} {...props}>{children}</span>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  GearSix: (props: any) => <span data-testid="icon-gear" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  Megaphone: (props: any) => <span data-testid="icon-megaphone" {...props} />,
  ChartLine: (props: any) => <span data-testid="icon-chart-line" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
  Translate: (props: any) => <span data-testid="icon-translate" {...props} />,
  PresentationChart: (props: any) => <span data-testid="icon-presentation" {...props} />,
  Gauge: (props: any) => <span data-testid="icon-gauge" {...props} />,
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
  ClockCounterClockwise: (props: any) => <span data-testid="icon-clock" {...props} />,
  Database: (props: any) => <span data-testid="icon-database" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  Wheelchair: (props: any) => <span data-testid="icon-wheelchair" {...props} />,
  Wrench: (props: any) => <span data-testid="icon-wrench" {...props} />,
  CurrencyDollar: (props: any) => <span data-testid="icon-currency" {...props} />,
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  PuzzlePiece: (props: any) => <span data-testid="icon-puzzle" {...props} />,
  GraphicsCard: (props: any) => <span data-testid="icon-graphql" {...props} />,
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  LockKey: (props: any) => <span data-testid="icon-lock-key" {...props} />,
  MapTrifold: (props: any) => <span data-testid="icon-map-trifold" {...props} />,
  ArrowsClockwise: (props: any) => <span data-testid="icon-arrows-clockwise" {...props} />,
  List: (props: any) => <span data-testid="icon-list" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  ArrowSquareOut: (props: any) => <span data-testid="icon-arrow-square-out" {...props} />,
}));

import { AdminPage } from './Admin';

describe('AdminPage', () => {
  it('renders Admin heading', () => {
    render(<AdminPage />);
    // "Administration" appears in both mobile header and desktop sidebar.
    expect(screen.getAllByText('Administration').length).toBeGreaterThan(0);
  });

  it('renders the subtitle', () => {
    render(<AdminPage />);
    expect(screen.getAllByText('Manage your ParkHub instance').length).toBeGreaterThan(0);
  });

  it('renders all navigation links inside the sidebar', () => {
    render(<AdminPage />);
    const nav = screen.getAllByLabelText('Admin navigation')[0]!;
    const scoped = within(nav);
    for (const label of [
      'Overview', 'Settings', 'Users', 'Parking Lots', 'Announcements',
      'Reports', 'Translations', 'Analytics', 'Rate Limits', 'Tenants',
      // Modules & Features is the feature the user specifically asked about
      'Modules & Features',
    ]) {
      expect(scoped.getByRole('link', { name: new RegExp(`^${label}$`) })).toBeInTheDocument();
    }
  });

  it('renders navigation links with correct paths', () => {
    render(<AdminPage />);
    const nav = screen.getAllByLabelText('Admin navigation')[0]!;
    const scoped = within(nav);
    const pairs: Array<[string, string]> = [
      ['Overview', '/admin'],
      ['Settings', '/admin/settings'],
      ['Users', '/admin/users'],
      ['Parking Lots', '/admin/lots'],
      ['Announcements', '/admin/announcements'],
      ['Reports', '/admin/reports'],
      ['Translations', '/admin/translations'],
      ['Analytics', '/admin/analytics'],
      ['Rate Limits', '/admin/rate-limits'],
      ['Tenants', '/admin/tenants'],
      ['Modules & Features', '/admin/modules'],
    ];
    for (const [label, path] of pairs) {
      const link = scoped.getByRole('link', { name: new RegExp(`^${label}$`) });
      expect(link).toHaveAttribute('href', path);
    }
  });

  it('renders the outlet for child routes', () => {
    render(<AdminPage />);
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });
});
