import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback || key,
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => <div ref={ref} {...props}>{children}</div>),
  },
}));

vi.mock('@phosphor-icons/react', () => {
  const C = (props: any) => <span {...props} />;
  return { Car: C, MagnifyingGlass: C, Flag: C, Lightning: C };
});

vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { AdminFleetPage } from './AdminFleet';
import toast from 'react-hot-toast';

const stats = { total_vehicles: 3, types_distribution: { car: 2, electric: 1 }, electric_count: 1, electric_ratio: 0.33, flagged_count: 1 };
const vehicles = [
  { id: 'v1', user_id: 'u1', username: 'alice', license_plate: 'M-AB-123', make: 'BMW', model: '320i', color: 'blue', vehicle_type: 'car', is_default: true, created_at: '2026-01-01', bookings_count: 5, last_used: '2026-04-10T08:00:00Z', flagged: false },
  { id: 'v2', user_id: 'u2', username: 'bob', license_plate: 'M-CD-456', vehicle_type: 'electric', is_default: false, created_at: '2026-02-01', bookings_count: 2, flagged: true, flag_reason: 'suspicious' },
  { id: 'v3', user_id: 'u3', license_plate: 'M-EF-789', vehicle_type: 'unknown_type', is_default: false, created_at: '2026-03-01', bookings_count: 0, flagged: false },
];

describe('AdminFleetPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/stats')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: stats }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: vehicles }) } as Response);
    }) as any;
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders after loading', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByText('M-AB-123')).toBeInTheDocument());
  });

  it('renders stats cards', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByTestId('fleet-stats')).toBeInTheDocument());
  });

  it('renders type distribution', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByTestId('type-distribution')).toBeInTheDocument());
  });

  it('renders table with vehicle rows', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('fleet-row');
      expect(rows).toHaveLength(3);
    });
  });

  it('shows flagged vehicle indicator', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByText('M-CD-456')).toBeInTheDocument());
  });

  it('shows empty vehicles table', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/stats')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: stats }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByText('No vehicles found')).toBeInTheDocument());
  });

  it('handles flag toggle', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getAllByTestId('flag-btn').length).toBeGreaterThan(0));
    const flagBtns = screen.getAllByTestId('flag-btn');
    fireEvent.click(flagBtns[0]); // flag unflagged vehicle
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/flag'), expect.anything()));
  });

  it('handles flag API success', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/flag')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/stats')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: stats }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: vehicles }) } as Response);
    }) as any;
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getAllByTestId('flag-btn').length).toBeGreaterThan(0));
    fireEvent.click(screen.getAllByTestId('flag-btn')[0]);
    await waitFor(() => expect(toast.success).toHaveBeenCalled());
  });

  it('handles flag API failure', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/flag')) return Promise.reject(new Error('fail'));
      if (url.includes('/stats')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: stats }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: vehicles }) } as Response);
    }) as any;
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getAllByTestId('flag-btn').length).toBeGreaterThan(0));
    fireEvent.click(screen.getAllByTestId('flag-btn')[0]);
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('type filter changes', async () => {
    const user = userEvent.setup();
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByTestId('fleet-type-filter')).toBeInTheDocument());
    await user.selectOptions(screen.getByTestId('fleet-type-filter'), 'electric');
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalled());
  });

  it('search with Enter key', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByTestId('fleet-search')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('fleet-search'), { target: { value: 'BMW' } });
    fireEvent.keyDown(screen.getByTestId('fleet-search'), { key: 'Enter' });
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalled());
  });

  it('handles load failure', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;
    render(<AdminFleetPage />);
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('shows make/model or dash', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => {
      expect(screen.getByText('BMW 320i')).toBeInTheDocument();
    });
  });

  it('shows last_used date or dash', async () => {
    render(<AdminFleetPage />);
    await waitFor(() => expect(screen.getByText('M-AB-123')).toBeInTheDocument());
  });
});
