import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'fleet.title': 'Fleet Management',
        'fleet.subtitle': 'All vehicles across all users',
        'fleet.totalVehicles': 'Total Vehicles',
        'fleet.electricCount': 'Electric',
        'fleet.electricRatio': 'Electric Ratio',
        'fleet.flaggedCount': 'Flagged',
        'fleet.byType': 'By Type',
        'fleet.search': 'Search plate, make, model...',
        'fleet.allTypes': 'All Types',
        'fleet.colPlate': 'Plate',
        'fleet.colType': 'Type',
        'fleet.colOwner': 'Owner',
        'fleet.colMakeModel': 'Make/Model',
        'fleet.colBookings': 'Bookings',
        'fleet.colLastUsed': 'Last Used',
        'fleet.colActions': 'Actions',
        'fleet.empty': 'No vehicles found',
        'fleet.flag': 'Flag',
        'fleet.unflag': 'Unflag',
        'common.error': 'Error',
      };
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  Flag: (props: any) => <span data-testid="icon-flag" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
}));

import { AdminFleetPage } from './AdminFleet';

const sampleFleet = [
  { id: 'v1', user_id: 'u1', username: 'alice', license_plate: 'AB-123', make: 'Tesla', model: 'Model 3', color: 'white', vehicle_type: 'electric', is_default: true, created_at: '2026-01-01T00:00:00Z', bookings_count: 5, last_used: '2026-03-20T10:00:00Z', flagged: false, flag_reason: null },
  { id: 'v2', user_id: 'u2', username: 'bob', license_plate: 'CD-456', make: 'BMW', model: '3er', color: 'black', vehicle_type: 'car', is_default: true, created_at: '2026-02-01T00:00:00Z', bookings_count: 2, last_used: null, flagged: true, flag_reason: 'stolen' },
];

const sampleStats = {
  total_vehicles: 2,
  types_distribution: { electric: 1, car: 1 },
  electric_count: 1,
  electric_ratio: 0.5,
  flagged_count: 1,
};

describe('AdminFleetPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleStats }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleFleet }) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title after loading', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('Fleet Management')).toBeInTheDocument();
    });
  });

  it('renders stats cards', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByTestId('fleet-stats')).toBeInTheDocument();
      expect(screen.getByText('Total Vehicles')).toBeInTheDocument();
    });
  });

  it('renders vehicle rows in the table', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('fleet-row');
      expect(rows).toHaveLength(2);
    });
  });

  it('shows type distribution badges', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByTestId('type-distribution')).toBeInTheDocument();
    });
  });

  it('renders search and filter controls', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByTestId('fleet-search')).toBeInTheDocument();
      expect(screen.getByTestId('fleet-type-filter')).toBeInTheDocument();
    });
  });

  it('shows empty state when no vehicles', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { ...sampleStats, total_vehicles: 0, types_distribution: {} } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;

    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('No vehicles found')).toBeInTheDocument();
    });
  });
});
