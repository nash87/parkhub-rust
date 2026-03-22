import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
  Outlet: () => <div data-testid="outlet">Outlet Content</div>,
  useLocation: () => ({ pathname: '/admin' }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
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
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
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
}));

import { AdminPage } from './Admin';

describe('AdminPage', () => {
  it('renders Admin heading', () => {
    render(<AdminPage />);
    expect(screen.getByText('Administration')).toBeInTheDocument();
  });

  it('renders the subtitle', () => {
    render(<AdminPage />);
    expect(screen.getByText('Manage your ParkHub instance')).toBeInTheDocument();
  });

  it('renders all tab navigation links', () => {
    render(<AdminPage />);
    expect(screen.getByText('Overview')).toBeInTheDocument();
    expect(screen.getByText('Settings')).toBeInTheDocument();
    expect(screen.getByText('Users')).toBeInTheDocument();
    expect(screen.getByText('Parking Lots')).toBeInTheDocument();
    expect(screen.getByText('Announcements')).toBeInTheDocument();
    expect(screen.getByText('Reports')).toBeInTheDocument();
    expect(screen.getByText('Translations')).toBeInTheDocument();
    expect(screen.getByText('Analytics')).toBeInTheDocument();
  });

  it('renders tab links with correct paths', () => {
    render(<AdminPage />);
    expect(screen.getByText('Overview').closest('a')).toHaveAttribute('href', '/admin');
    expect(screen.getByText('Settings').closest('a')).toHaveAttribute('href', '/admin/settings');
    expect(screen.getByText('Users').closest('a')).toHaveAttribute('href', '/admin/users');
    expect(screen.getByText('Parking Lots').closest('a')).toHaveAttribute('href', '/admin/lots');
    expect(screen.getByText('Announcements').closest('a')).toHaveAttribute('href', '/admin/announcements');
    expect(screen.getByText('Reports').closest('a')).toHaveAttribute('href', '/admin/reports');
    expect(screen.getByText('Translations').closest('a')).toHaveAttribute('href', '/admin/translations');
    expect(screen.getByText('Analytics').closest('a')).toHaveAttribute('href', '/admin/analytics');
  });

  it('renders the outlet for child routes', () => {
    render(<AdminPage />);
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });
});
