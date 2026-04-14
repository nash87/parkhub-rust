import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

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

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
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

  it('flags an unflagged vehicle', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => screen.getAllByTestId('fleet-row'));

    const flagBtns = screen.getAllByTestId('flag-btn');
    // First vehicle (alice's Tesla) is not flagged
    fireEvent.click(flagBtns[0]);
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/fleet/v1/flag',
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"flagged":true'),
        }),
      );
    });
  });

  it('unflags a flagged vehicle', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => screen.getAllByTestId('fleet-row'));

    const flagBtns = screen.getAllByTestId('flag-btn');
    // Second vehicle (bob's BMW) is flagged
    fireEvent.click(flagBtns[1]);
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/fleet/v2/flag',
        expect.objectContaining({
          method: 'PUT',
          body: expect.stringContaining('"flagged":false'),
        }),
      );
    });
  });

  it('handles flag error', async () => {
    const toast = (await import('react-hot-toast')).default;
    let callCount = 0;
    global.fetch = vi.fn((url: string) => {
      callCount++;
      if (url.includes('/stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleStats }) } as Response);
      }
      if (url.includes('/flag')) {
        return Promise.reject(new Error('Flag failed'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleFleet }) } as Response);
    }) as any;

    render(<AdminFleetPage />);
    await waitFor(() => screen.getAllByTestId('flag-btn'));
    fireEvent.click(screen.getAllByTestId('flag-btn')[0]);
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Error');
    });
  });

  it('filters by type dropdown', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => screen.getByTestId('fleet-type-filter'));

    fireEvent.change(screen.getByTestId('fleet-type-filter'), { target: { value: 'electric' } });
    // The loadData useCallback depends on typeFilter, so changing it triggers re-fetch
    await waitFor(() => {
      const calls = (global.fetch as any).mock.calls;
      const matchingCalls = calls.filter((c: any[]) => typeof c[0] === 'string' && c[0].includes('type=electric'));
      expect(matchingCalls.length).toBeGreaterThan(0);
    });
  });

  it('searches on Enter key', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => screen.getByTestId('fleet-search'));

    const searchInput = screen.getByTestId('fleet-search');
    fireEvent.change(searchInput, { target: { value: 'Tesla' } });
    fireEvent.keyDown(searchInput, { key: 'Enter' });
    // The keyDown handler calls loadData() directly, and search state change also triggers useEffect
    await waitFor(() => {
      const calls = (global.fetch as any).mock.calls;
      const matchingCalls = calls.filter((c: any[]) => typeof c[0] === 'string' && c[0].includes('search=Tesla'));
      expect(matchingCalls.length).toBeGreaterThan(0);
    });
  });

  it('shows flagged vehicle with flag icon and unflag button text', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      // Bob's car is flagged
      const flagBtns = screen.getAllByTestId('flag-btn');
      expect(flagBtns[1].textContent).toContain('Unflag');
      expect(flagBtns[0].textContent).toContain('Flag');
    });
  });

  it('shows vehicle details (make, model, plate, owner)', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('AB-123')).toBeInTheDocument();
      expect(screen.getByText('CD-456')).toBeInTheDocument();
      expect(screen.getByText('alice')).toBeInTheDocument();
      expect(screen.getByText('bob')).toBeInTheDocument();
      expect(screen.getByText('Tesla Model 3')).toBeInTheDocument();
      expect(screen.getByText('BMW 3er')).toBeInTheDocument();
    });
  });

  it('shows electric ratio percentage', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('50%')).toBeInTheDocument();
    });
  });

  it('handles fetch error on initial load', async () => {
    const toast = (await import('react-hot-toast')).default;
    global.fetch = vi.fn(() => Promise.reject(new Error('Network'))) as any;
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Error');
    });
  });

  it('shows loading skeleton initially', () => {
    global.fetch = vi.fn(() => new Promise(() => {})) as any; // never resolves
    render(<AdminFleetPage />);
    expect(document.querySelector('.skeleton')).toBeInTheDocument();
  });

  it('shows bookings count for vehicles', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('5')).toBeInTheDocument(); // v1 bookings_count
    });
  });

  it('shows last used date for vehicles', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      // v1 has last_used, v2 does not
      const rows = screen.getAllByTestId('fleet-row');
      expect(rows[1].textContent).toContain('-'); // no last_used
    });
  });
});
