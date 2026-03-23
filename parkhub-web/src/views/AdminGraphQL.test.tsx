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
    div: React.forwardRef(({ children, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => {
  const icon = (props: any) => <span {...props} />;
  return {
    ChartBar: icon, GearSix: icon, Users: icon, Megaphone: icon,
    ChartLine: icon, MapPin: icon, Translate: icon, PresentationChart: icon,
    Gauge: icon, Buildings: icon, ClockCounterClockwise: icon, Database: icon,
    Car: icon, Wheelchair: icon, Wrench: icon, CurrencyDollar: icon,
    UserPlus: icon, Lightning: icon, PuzzlePiece: icon, GraphicsCard: icon,
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
    expect(screen.getByText('GraphQL')).toBeInTheDocument();
  });

  it('renders Plugins link in admin nav', () => {
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <AdminPage />
      </MemoryRouter>
    );
    expect(screen.getByText('Plugins')).toBeInTheDocument();
  });

  it('GraphQL link points to playground endpoint', () => {
    render(
      <MemoryRouter initialEntries={['/admin']}>
        <AdminPage />
      </MemoryRouter>
    );
    const link = screen.getByText('GraphQL').closest('a');
    expect(link).toBeTruthy();
    expect(link?.getAttribute('href')).toBe('/api/v1/graphql/playground');
  });
});
